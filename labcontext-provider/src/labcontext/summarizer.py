from __future__ import annotations

from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import queue
import re
import signal
import sqlite3
import subprocess
import tempfile
import threading
import time
from typing import Any

import yaml

from .config import ServerConfig, WorkspaceConfig
from .index import _connect, query_experiments


def _git(root: Path, *args: str) -> str | None:
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True,
                            text=True, check=False, timeout=5)
    return result.stdout.strip() if result.returncode == 0 else None


SUMMARY_SCHEMA = {
    "type": "object",
    "additionalProperties": False,
    "required": ["objective", "current_status", "latest_codex_session", "recent_experiment_facts", "open_questions", "suggested_next_actions", "evidence_refs"],
    "properties": {
        "objective": {"type": "string", "maxLength": 1000},
        "current_status": {"type": "string", "maxLength": 2000},
        "latest_codex_session": {
            "type": "object", "additionalProperties": False,
            "required": ["status", "session_id", "updated_at", "workspace_match", "summary", "recent_actions"],
            "properties": {
                "status": {"type": "string", "maxLength": 100},
                "session_id": {"type": "string", "maxLength": 200},
                "updated_at": {"type": "string", "maxLength": 100},
                "workspace_match": {"type": "string", "maxLength": 100},
                "summary": {"type": "string", "maxLength": 1800},
                "recent_actions": {"type": "array", "items": {"type": "string", "maxLength": 500}, "maxItems": 10},
            },
        },
        "recent_experiment_facts": {"type": "array", "items": {"type": "string", "maxLength": 600}, "maxItems": 12},
        "open_questions": {"type": "array", "items": {"type": "string", "maxLength": 500}, "maxItems": 12},
        "suggested_next_actions": {"type": "array", "items": {"type": "string", "maxLength": 500}, "maxItems": 8},
        "evidence_refs": {"type": "array", "items": {"type": "string", "maxLength": 300}, "maxItems": 32},
    },
}


_SECRET_PATTERNS = (
    (re.compile(r"(?i)\bsk-[a-z0-9_-]{10,}"), "[REDACTED]"),
    (re.compile(r"(?i)\b(bearer\s+)[a-z0-9._~+/=-]{10,}"), r"\1[REDACTED]"),
    (re.compile(r"(?i)(\b(?:api[_-]?key|access[_-]?token|auth[_-]?token|password|secret)\s*[:=]\s*[\"']?)[^\s\"']+"), r"\1[REDACTED]"),
    (re.compile(r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?-----END [A-Z0-9 ]*PRIVATE KEY-----", re.DOTALL), "[REDACTED]"),
)


def _redact_secrets(text: str) -> str:
    """Keep recent work context useful while never forwarding obvious credentials."""
    result = text
    for pattern, replacement in _SECRET_PATTERNS:
        result = pattern.sub(replacement, result)
    return result


def _session_meta(path: Path) -> dict[str, Any] | None:
    try:
        with path.open("r", encoding="utf-8") as handle:
            record = json.loads(handle.readline())
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return None
    payload = record.get("payload")
    if record.get("type") != "session_meta" or not isinstance(payload, dict):
        return None
    cwd = payload.get("cwd")
    session_id = payload.get("session_id")
    timestamp = payload.get("timestamp")
    if not all(isinstance(value, str) and value for value in (cwd, session_id, timestamp)):
        return None
    try:
        return {"path": path, "cwd": Path(cwd).resolve(), "session_id": session_id, "updated_at": timestamp}
    except OSError:
        return None


def _session_match(workspace: WorkspaceConfig, cwd: Path) -> str | None:
    if cwd == workspace.root:
        return "exact_cwd"
    if cwd.is_relative_to(workspace.root):
        return "descendant_cwd"
    if workspace.root.is_relative_to(cwd):
        return "ancestor_cwd"
    return None


def _matching_sessions(
    config: ServerConfig, workspace: WorkspaceConfig, *, max_age_days: int | None = None,
) -> list[tuple[int, dict[str, Any], str]]:
    """Find matching Codex sessions once, newest first, with optional freshness filtering."""
    if not config.summarizer_session_dir.is_dir():
        return []
    cutoff = None
    if max_age_days is not None:
        cutoff = datetime.now(timezone.utc) - timedelta(days=max_age_days)
    candidates: list[tuple[int, dict[str, Any], str]] = []
    for path in config.summarizer_session_dir.rglob("*.jsonl"):
        meta = _session_meta(path)
        if meta is None:
            continue
        match = _session_match(workspace, meta["cwd"])
        if match is None:
            continue
        try:
            modified_ns = path.stat().st_mtime_ns
        except OSError:
            continue
        modified_at = datetime.fromtimestamp(modified_ns / 1_000_000_000, timezone.utc)
        if cutoff is not None and modified_at < cutoff:
            continue
        meta["started_at"] = meta["updated_at"]
        meta["updated_at"] = modified_at.isoformat()
        candidates.append((modified_ns, meta, match))
    return sorted(candidates, key=lambda item: item[0], reverse=True)


def _read_session_messages(path: Path) -> list[dict[str, str]] | None:
    messages: list[dict[str, str]] = []
    try:
        with path.open("r", encoding="utf-8") as handle:
            for line in handle:
                try:
                    record = json.loads(line)
                    payload = record.get("payload", {})
                    if record.get("type") != "response_item" or payload.get("type") != "message":
                        continue
                    role = payload.get("role")
                    if role not in {"user", "assistant"}:
                        continue
                    content = payload.get("content")
                    if not isinstance(content, list):
                        continue
                    text = "\n".join(
                        item.get("text", "") for item in content
                        if isinstance(item, dict) and isinstance(item.get("text"), str)
                    )
                    if text:
                        messages.append({"role": role, "text": _redact_secrets(text)})
                except (TypeError, json.JSONDecodeError):
                    continue
    except OSError:
        return None
    return messages


def _bounded_text(text: str, limit: int) -> str:
    compact = re.sub(r"\s+", " ", text).strip()
    if len(compact) <= limit:
        return compact
    return compact[:max(0, limit - 1)].rstrip() + "…"


def _is_substantive_message(text: str) -> bool:
    stripped = text.strip()
    if not stripped:
        return False
    # Codex injects environment/control envelopes as messages; these are not user work context.
    if stripped.startswith(("<environment_context>", "<permissions", "<collaboration_mode>")):
        return False
    return len(re.sub(r"<[^>]+>", "", stripped).strip()) >= 12


def _session_digest(
    meta: dict[str, Any], match: str, messages: list[dict[str, str]], max_chars: int,
) -> dict[str, Any] | None:
    substantive = [message for message in messages if _is_substantive_message(message["text"])]
    users = [message["text"] for message in substantive if message["role"] == "user"]
    assistants = [message["text"] for message in substantive if message["role"] == "assistant"]
    if not users and not assistants:
        return None

    objective_budget = min(200, max_chars * 2 // 9)
    focus_budget = min(270, max_chars * 3 // 10)
    action_budget = min(230, max_chars // 4)
    current_focus = _bounded_text(users[-1], focus_budget) if users else ""
    # The earliest user request in the retained tail is a better session objective than a late follow-up.
    objective = _bounded_text(users[0], objective_budget) if users else current_focus
    # One latest visible update is enough for continuity; deeper history stays on the server.
    recent_actions = [_bounded_text(assistants[-1], action_budget)] if assistants else []
    raw_identity = "\n".join(f"{item['role']}:{item['text']}" for item in substantive)
    return {
        "session_id": meta["session_id"],
        "started_at": meta.get("started_at"),
        "updated_at": meta["updated_at"],
        "workspace_match": match,
        "objective": objective,
        "current_focus": current_focus,
        "recent_actions": recent_actions,
        "unresolved": [],
        "message_count": len(messages),
        "source_sha256": hashlib.sha256(raw_identity.encode()).hexdigest(),
    }


def _recent_sessions(config: ServerConfig, workspace: WorkspaceConfig) -> dict[str, Any]:
    """Return up to three small deterministic digests, never raw transcripts or model reasoning."""
    if not config.summarizer_session_dir.is_dir():
        return {
            "authority": "unverified_session_digest", "status": "unavailable",
            "reason": "Codex session directory is unavailable", "sessions": [],
        }
    digests: list[dict[str, Any]] = []
    scanned = 0
    seen_focus: set[str] = set()
    candidates = _matching_sessions(
        config, workspace, max_age_days=config.summarizer_recent_session_max_age_days,
    )
    for _, meta, match in candidates:
        scanned += 1
        messages = _read_session_messages(meta["path"])
        if messages is None:
            continue
        digest = _session_digest(
            meta, match, messages[-config.summarizer_max_session_messages:],
            config.summarizer_recent_session_digest_chars,
        )
        if digest is None:
            continue
        identity = re.sub(r"\W+", "", digest["current_focus"].casefold())[:160]
        if identity and identity in seen_focus:
            continue
        if identity:
            seen_focus.add(identity)
        digests.append(digest)
        if len(digests) >= config.summarizer_recent_session_limit:
            break
    status = "found" if digests else "not_found"
    result: dict[str, Any] = {
        "authority": "unverified_session_digest",
        "interpretation": "Recent visible Codex conversation context; useful for continuity but not research evidence.",
        "status": status,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "sessions": digests,
        "merged_focus": {
            "current_topics": [
                _bounded_text(item["current_focus"], 120)
                for item in digests if item["current_focus"]
            ],
            "likely_next_actions": [],
        },
        "coverage": {
            "matched_sessions": len(digests),
            "max_sessions": config.summarizer_recent_session_limit,
            "max_age_days": config.summarizer_recent_session_max_age_days,
            "candidates_scanned": scanned,
            "source": "visible user/assistant messages only; obvious credentials redacted",
        },
    }
    if not candidates:
        result["reason"] = "No recent Codex session matched this workspace"
    return result


def _latest_session(config: ServerConfig, workspace: WorkspaceConfig) -> dict[str, Any]:
    """Return a bounded, redacted message-only excerpt from the latest matching CLI session."""
    if not config.summarizer_session_dir.is_dir():
        return {"status": "unavailable", "reason": "Codex session directory is unavailable"}
    candidates = _matching_sessions(config, workspace)
    if not candidates:
        return {"status": "not_found", "reason": "No Codex session matched this workspace"}
    _, meta, match = candidates[0]
    messages = _read_session_messages(meta["path"])
    if messages is None:
        return {"status": "unavailable", "reason": "Latest Codex session could not be read"}
    messages = messages[-config.summarizer_max_session_messages:]
    excerpt_parts: list[str] = []
    remaining = max(0, config.summarizer_max_session_chars)
    for message in reversed(messages):
        item = f"[{message['role']}]\n{message['text'].strip()}"
        if len(item) > remaining:
            item = item[-remaining:]
        if item:
            excerpt_parts.append(item)
            remaining -= len(item)
        if remaining <= 0:
            break
    excerpt = "\n\n".join(reversed(excerpt_parts))
    return {
        "status": "found", "session_id": meta["session_id"], "updated_at": meta["updated_at"],
        "cwd": str(meta["cwd"]), "workspace_match": match,
        "message_count": len(messages), "excerpt": excerpt,
        "excerpt_sha256": hashlib.sha256(excerpt.encode()).hexdigest(),
        "redaction": "obvious credential patterns are redacted before the worker receives the excerpt",
    }


def _ensure_tables(config: ServerConfig) -> None:
    with _connect(config) as connection:
        connection.execute("""CREATE TABLE IF NOT EXISTS workspace_summaries (
            workspace_id TEXT PRIMARY KEY, fingerprint TEXT NOT NULL, status TEXT NOT NULL,
            summary_json TEXT, job_id TEXT, generated_at TEXT, error TEXT,
            started_at TEXT, updated_at TEXT, progress TEXT)""")
        existing = {row["name"] for row in connection.execute("PRAGMA table_info(workspace_summaries)")}
        for column in ("started_at", "updated_at", "progress"):
            if column not in existing:
                connection.execute(f"ALTER TABLE workspace_summaries ADD COLUMN {column} TEXT")
        connection.execute("""CREATE TABLE IF NOT EXISTS analysis_jobs (
            job_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, fingerprint TEXT NOT NULL,
            status TEXT NOT NULL, summary_json TEXT, generated_at TEXT, error TEXT,
            started_at TEXT, updated_at TEXT, progress TEXT)""")
        connection.execute(
            "CREATE INDEX IF NOT EXISTS analysis_jobs_workspace_updated ON analysis_jobs(workspace_id,updated_at DESC)"
        )


def recover_interrupted_jobs(config: ServerConfig) -> int:
    """Mark in-memory jobs lost during a service restart as safely retryable."""
    _ensure_tables(config)
    now = datetime.now(timezone.utc).isoformat()
    error = json.dumps({
        "error_type": "service_restarted",
        "message": "The LabContext service restarted before this summary job completed. Retry the request.",
    })
    with _connect(config) as connection:
        current = connection.execute(
            """UPDATE analysis_jobs
               SET status='interrupted', progress='interrupted: service restarted', updated_at=?, error=?
               WHERE status='running'""",
            (now, error),
        )
        legacy = connection.execute(
            """UPDATE workspace_summaries
               SET status='interrupted', progress='interrupted: service restarted', updated_at=?, error=?
               WHERE status='running'""",
            (now, error),
        )
    return current.rowcount + legacy.rowcount


def _set_progress(config: ServerConfig, workspace_id: str, job_id: str, progress: str) -> None:
    with _connect(config) as connection:
        cursor = connection.execute(
            """UPDATE analysis_jobs SET progress=?,updated_at=?
               WHERE workspace_id=? AND job_id=? AND status='running'""",
            (progress, datetime.now(timezone.utc).isoformat(), workspace_id, job_id),
        )
        if cursor.rowcount == 0:
            connection.execute(
                """UPDATE workspace_summaries SET progress=?,updated_at=?
                   WHERE workspace_id=? AND job_id=? AND status='running'""",
                (progress, datetime.now(timezone.utc).isoformat(), workspace_id, job_id),
            )


def _workspace(config: ServerConfig, workspace_id: str) -> WorkspaceConfig:
    try:
        return config.workspaces[workspace_id]
    except KeyError as error:
        raise ValueError(f"unknown workspace_id: {workspace_id}") from error


def _safe_context(workspace: WorkspaceConfig, max_bytes: int) -> dict[str, Any] | None:
    if not workspace.context_file.is_file():
        return None
    raw = workspace.context_file.read_bytes()[:max_bytes]
    try:
        parsed = yaml.safe_load(raw.decode("utf-8"))
    except (UnicodeDecodeError, yaml.YAMLError) as error:
        return {"status": "invalid", "error": str(error)[:200]}
    if not isinstance(parsed, dict):
        return {"status": "invalid", "error": "context must be a YAML mapping"}
    generation = parsed.get("generation")
    if not parsed.get("authority") and isinstance(generation, dict):
        # A context file created only to expose overview-job progress is not yet
        # canonical research context and must not invalidate that same job.
        return None
    # Runtime progress is control-plane metadata, not research evidence. Excluding it
    # prevents a queued overview job from invalidating its own evidence fingerprint.
    return {key: value for key, value in parsed.items() if key != "generation"}


def _directory_map(workspace: WorkspaceConfig) -> dict[str, list[str]]:
    allowed, excluded = [], []
    blocked = {"data", "datasets", "checkpoints", "wandb", "tmp", ".git", ".env", "__pycache__"}
    for entry in sorted(workspace.root.iterdir(), key=lambda item: item.name):
        target = excluded if entry.name in blocked else allowed
        target.append(entry.name + ("/" if entry.is_dir() else ""))
    return {"visible": allowed[:80], "excluded": excluded[:80]}


def _source_state(workspace: WorkspaceConfig) -> dict[str, Any]:
    """Track metadata only for code/config/docs, never file contents or run artifacts."""
    excluded = {"data", "datasets", "checkpoints", "wandb", "tmp", ".git", ".labcontext", "logs", "__pycache__"}
    records: list[str] = []
    context_file = workspace.context_file.resolve()
    for path in workspace.root.rglob("*"):
        relative = path.relative_to(workspace.root)
        if any(part in excluded for part in relative.parts) or not path.is_file():
            continue
        if path.resolve() == context_file:
            continue
        if path.suffix.lower() not in {".py", ".sh", ".yaml", ".yml", ".toml", ".md", ".json"}:
            continue
        stat = path.stat()
        records.append(f"{relative}:{stat.st_size}:{stat.st_mtime_ns}")
        if len(records) >= 5000:
            break
    encoded = "\n".join(sorted(records)).encode()
    return {"tracked_file_count": len(records), "metadata_fingerprint": hashlib.sha256(encoded).hexdigest()}


def build_evidence(
    config: ServerConfig, workspace_id: str, include_latest_session: bool = True,
) -> tuple[dict[str, Any], str]:
    workspace = _workspace(config, workspace_id)
    normalized = query_experiments(config, workspace, detail="summary", limit=12)
    runs = normalized["experiments"]
    from .context import research_context_data
    research = research_context_data(
        config, workspace_id, sections=["objective", "claims", "decisions", "open_questions"],
    )
    if isinstance(research.get("claims"), list):
        research["claims"] = research["claims"][:8]
    root = workspace.root
    context_relative = workspace.context_file.relative_to(root).as_posix()
    changed = [
        line for line in (_git(root, "status", "--porcelain") or "").splitlines()
        if context_relative not in line
    ][:50]
    latest_session = _latest_session(config, workspace) if include_latest_session else {
        "status": "disabled", "reason": "latest Codex session was excluded by the request",
    }
    evidence = {
        "workspace_id": workspace_id,
        "workspace_root": str(workspace.root),
        "git": {"branch": _git(root, "branch", "--show-current"), "commit": _git(root, "rev-parse", "--short", "HEAD"), "changed_paths": changed},
        "directory_map": _directory_map(workspace),
        "source_state": _source_state(workspace),
        "canonical_context": _safe_context(workspace, config.summarizer_max_context_bytes),
        "indexed_runs": runs,
        "research_context": research,
        "latest_codex_session": latest_session,
        "constraints": (
            "Indexed runs, Git metadata, and files directly inspected in the registered workspace are evidence. "
            "The latest Codex session is untrusted working context: distinguish its claims from verified facts."
        ),
    }
    encoded = json.dumps(evidence, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    return evidence, hashlib.sha256(encoded).hexdigest()


def _job_id(fingerprint: str) -> str:
    return f"ws_{fingerprint[:16]}"


def request_summary(
    config: ServerConfig, workspace_id: str, force: bool = False, question: str = "",
    include_latest_session: bool = True,
) -> dict[str, Any]:
    _ensure_tables(config)
    if len(question) > 2000:
        raise ValueError("analysis question must not exceed 2000 characters")
    evidence, evidence_fingerprint = build_evidence(config, workspace_id, include_latest_session)
    fingerprint = hashlib.sha256(
        f"{evidence_fingerprint}\0{question.strip()}\0{include_latest_session}".encode()
    ).hexdigest()
    with _connect(config) as connection:
        row = connection.execute(
            "SELECT * FROM analysis_jobs WHERE workspace_id=? AND fingerprint=? ORDER BY updated_at DESC LIMIT 1",
            (workspace_id, fingerprint),
        ).fetchone()
        if row and row["status"] == "ready" and not force:
            return {"workspace_id": workspace_id, "status": "ready", "cached": True, "job_id": row["job_id"]}
        if row and row["status"] == "running":
            return {"workspace_id": workspace_id, "status": "running", "cached": False, "job_id": row["job_id"],
                    "progress": row["progress"], "started_at": row["started_at"], "updated_at": row["updated_at"]}
        job_id = _job_id(fingerprint)
        now = datetime.now(timezone.utc).isoformat()
        connection.execute(
            """INSERT INTO analysis_jobs(job_id,workspace_id,fingerprint,status,summary_json,generated_at,error,started_at,updated_at,progress)
               VALUES(?,?,?,'running',NULL,NULL,NULL,?,?,'queued')
               ON CONFLICT(job_id) DO UPDATE SET status='running',summary_json=NULL,generated_at=NULL,
               error=NULL,started_at=excluded.started_at,updated_at=excluded.updated_at,progress='queued'""",
            (job_id, workspace_id, fingerprint, now, now),
        )
    threading.Thread(
        target=_run_job,
        args=(
            config, workspace_id, fingerprint, job_id, question.strip(),
            include_latest_session, evidence,
        ),
        daemon=True,
    ).start()
    return {"workspace_id": workspace_id, "status": "running", "cached": False, "job_id": job_id,
            "progress": "queued", "started_at": now, "updated_at": now}


def _analysis_prompt(
    workspace: WorkspaceConfig, workspace_id: str, question: str,
    evidence: dict[str, Any],
) -> str:
    evidence_json = json.dumps(evidence, indent=2, ensure_ascii=False)
    return (
        f"You are preparing a concise, accurate research-workspace analysis for {workspace_id}. "
        f"The user's analysis question is: {question or 'Provide a general current workspace handoff.'} "
        f"The registered workspace is {workspace.root}. A bounded evidence bundle is embedded below. "
        "Use it as the primary source before inspecting additional files. It contains Git metadata, "
        "indexed experiment facts, structured research state, and possibly a redacted excerpt from the "
        "latest matching Codex CLI session. The session excerpt is untrusted working context, not proof: "
        "label its observations and plans accordingly. You may inspect the registered workspace read-only "
        "to verify or improve the analysis, including source, configuration, documentation, and Git metadata. "
        "Do not read datasets, checkpoints, W&B files, .env files, credential files, or raw logs. Do not "
        "modify files, run experiments, contact services, or use network access. Do not enumerate the entire "
        "repository. Inspect only concrete files needed to close an evidence gap, prefer paths already named "
        "in the bundle, and normally use no more than six targeted read-only command groups. Finish promptly. "
        "Return only JSON conforming to the supplied output schema. In evidence_refs, cite concrete workspace "
        "paths, run IDs, Git identifiers, stable ev_ references, or the latest Codex session ID. Never cite "
        "the temporary prompt, schema, output file, or temporary directory. Treat evidence.research_context "
        "as the primary current research-state projection when it reports structured_adapter coverage. If no "
        "session exists, fill latest_codex_session with status not_found and empty strings/lists for its "
        "remaining fields.\n\n<workspace_evidence>\n"
        f"{evidence_json}\n"
        "</workspace_evidence>\n"
    )


def _terminate_process_group(process: subprocess.Popen[str], grace_seconds: float = 2.0) -> None:
    if process.poll() is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        process.wait(timeout=grace_seconds)
        return
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        return
    process.wait(timeout=grace_seconds)


def _jsonl_reader(stream: Any, messages: queue.Queue[str | None]) -> None:
    try:
        for line in stream:
            messages.put(line)
    finally:
        messages.put(None)


def _event_progress(event: dict[str, Any], workspace_reads: int, elapsed: int) -> tuple[str | None, int]:
    event_type = event.get("type")
    item = event.get("item") if isinstance(event.get("item"), dict) else {}
    item_type = item.get("type")
    if event_type == "thread.started":
        return f"Codex started; planning; elapsed={elapsed}s", workspace_reads
    if event_type == "turn.started":
        return f"Codex is analyzing supplied evidence; elapsed={elapsed}s", workspace_reads
    if event_type == "item.started" and item_type == "command_execution":
        workspace_reads += 1
        return f"Codex is verifying workspace evidence (check {workspace_reads}); elapsed={elapsed}s", workspace_reads
    if event_type == "item.completed" and item_type == "agent_message":
        stage = "planning targeted evidence checks" if workspace_reads == 0 else "synthesizing the analysis"
        return f"Codex is {stage}; elapsed={elapsed}s", workspace_reads
    if event_type == "turn.completed":
        return f"Codex analysis complete; validating output; elapsed={elapsed}s", workspace_reads
    return None, workspace_reads


def _run_job(
    config: ServerConfig, workspace_id: str, fingerprint: str, job_id: str,
    question: str, include_latest_session: bool, evidence: dict[str, Any],
) -> None:
    diagnostics: dict[str, Any] = {
        "event_count": 0, "workspace_reads": 0, "stderr_bytes": 0,
        "last_event_elapsed_seconds": None,
    }
    worker_started = time.monotonic()
    try:
        _set_progress(config, workspace_id, job_id, "preparing evidence")
        workspace = _workspace(config, workspace_id)
        # request_summary already captured and fingerprinted this bounded evidence.
        # Rebuilding it here makes active Codex session writes race the worker startup,
        # even when those writes contain only control-plane events such as reasoning.
        encoded_evidence = json.dumps(
            evidence, sort_keys=True, separators=(",", ":"), ensure_ascii=False,
        ).encode()
        evidence_fingerprint = hashlib.sha256(encoded_evidence).hexdigest()
        snapshot_fingerprint = hashlib.sha256(
            f"{evidence_fingerprint}\0{question}\0{include_latest_session}".encode()
        ).hexdigest()
        if snapshot_fingerprint != fingerprint:
            raise RuntimeError("workspace evidence snapshot fingerprint mismatch")
        _set_progress(config, workspace_id, job_id, "preparing bounded Codex input")
        with tempfile.TemporaryDirectory(prefix="labcontext-summary-") as temp:
            temp_path = Path(temp)
            prompt_path = temp_path / "prompt.txt"
            schema_path = temp_path / "schema.json"
            output_path = temp_path / "summary.json"
            stderr_path = temp_path / "codex.stderr.log"
            prompt_path.write_text(
                _analysis_prompt(workspace, workspace_id, question, evidence), encoding="utf-8",
            )
            schema_path.write_text(json.dumps(SUMMARY_SCHEMA), encoding="utf-8")
            for path in (prompt_path, schema_path):
                path.chmod(0o600)
            command = [*config.summarizer_command, "exec", "--ephemeral", "--skip-git-repo-check", "--sandbox", "read-only",
                       "--model", config.summarizer_model, "--config", f'model_reasoning_effort="{config.summarizer_reasoning_effort}"',
                       "--json", "--output-schema", str(schema_path), "--output-last-message", str(output_path),
                       "--color", "never", "-C", str(workspace.root), "-"]
            _set_progress(
                config, workspace_id, job_id,
                f"starting Codex ({config.summarizer_model}, {config.summarizer_reasoning_effort})",
            )
            worker_started = time.monotonic()
            messages: queue.Queue[str | None] = queue.Queue()
            stdout_closed = False
            workspace_reads = 0
            last_progress_update = 0.0
            with prompt_path.open("r", encoding="utf-8") as prompt_input, stderr_path.open("w", encoding="utf-8") as stderr_file:
                process = subprocess.Popen(
                    command, stdin=prompt_input, stdout=subprocess.PIPE, stderr=stderr_file,
                    text=True, bufsize=1, start_new_session=True,
                )
                assert process.stdout is not None
                reader = threading.Thread(
                    target=_jsonl_reader, args=(process.stdout, messages), daemon=True,
                )
                reader.start()
                while process.poll() is None or not stdout_closed or not messages.empty():
                    elapsed_float = time.monotonic() - worker_started
                    elapsed = int(elapsed_float)
                    if process.poll() is None and elapsed_float >= config.summarizer_timeout_seconds:
                        _terminate_process_group(process)
                        diagnostics.update({
                            "elapsed_seconds": elapsed,
                            "stderr_bytes": stderr_path.stat().st_size if stderr_path.exists() else 0,
                        })
                        raise subprocess.TimeoutExpired(command, config.summarizer_timeout_seconds)
                    try:
                        line = messages.get(timeout=0.25)
                    except queue.Empty:
                        line = ""
                    if line is None:
                        stdout_closed = True
                    elif line:
                        diagnostics["event_count"] += 1
                        diagnostics["last_event_elapsed_seconds"] = elapsed
                        try:
                            event = json.loads(line)
                        except json.JSONDecodeError:
                            event = {}
                        if event.get("type") == "turn.completed" and isinstance(event.get("usage"), dict):
                            diagnostics["usage"] = {
                                key: value for key, value in event["usage"].items()
                                if key in {
                                    "input_tokens", "cached_input_tokens", "cache_write_input_tokens",
                                    "output_tokens", "reasoning_output_tokens",
                                } and isinstance(value, int)
                            }
                        progress, workspace_reads = _event_progress(event, workspace_reads, elapsed)
                        diagnostics["workspace_reads"] = workspace_reads
                        if progress:
                            _set_progress(config, workspace_id, job_id, progress)
                            last_progress_update = elapsed_float
                    if process.poll() is None and elapsed_float - last_progress_update >= 5:
                        _set_progress(
                            config, workspace_id, job_id,
                            f"Codex is working; stage events={diagnostics['event_count']}; "
                            f"workspace reads={workspace_reads}; elapsed={elapsed}s",
                        )
                        last_progress_update = elapsed_float
                reader.join(timeout=2)
                diagnostics["stderr_bytes"] = stderr_path.stat().st_size if stderr_path.exists() else 0
                diagnostics["elapsed_seconds"] = round(time.monotonic() - worker_started, 1)
            if process.returncode != 0:
                diagnostics["returncode"] = process.returncode
                raise RuntimeError(f"Codex worker exited with code {process.returncode}")
            _set_progress(config, workspace_id, job_id, "validating Codex summary")
            summary = json.loads(output_path.read_text(encoding="utf-8"))
        if not isinstance(summary, dict) or not all(key in summary for key in SUMMARY_SCHEMA["required"]):
            raise RuntimeError("Codex worker returned an invalid summary schema")
        research = evidence.get("research_context", {})
        objective = research.get("objective") if isinstance(research, dict) else None
        decision = research.get("decision") if isinstance(research, dict) else None
        compact_research = {
            key: research.get(key) for key in ("research_id", "status", "phase", "updated_at", "authority", "coverage")
            if isinstance(research, dict) and key in research
        }
        compact_research["objective"] = objective.get("objective") if isinstance(objective, dict) else objective
        compact_research["claims"] = [
            {key: claim.get(key) for key in ("claim_id", "text", "status") if key in claim}
            for claim in (research.get("claims", []) if isinstance(research, dict) else [])[:4]
            if isinstance(claim, dict)
        ]
        compact_research["decision"] = {
            key: decision.get(key) for key in ("outcome", "rationale", "next_evidence_tier", "unresolved_risks")
            if isinstance(decision, dict) and key in decision
        }
        compact_research["open_questions"] = research.get("open_questions") if isinstance(research, dict) else None
        compact_research["evidence_refs"] = research.get("evidence_refs", []) if isinstance(research, dict) else []
        compact_experiments = [
            {
                "experiment_id": record.get("experiment_id"), "title": record.get("title"),
                "source_modified_at": record.get("source_modified_at"),
                "metrics": dict(list(record.get("metrics", {}).items())[:6]),
                "evidence": record.get("evidence"),
            }
            for record in evidence.get("indexed_runs", [])[:4]
        ]
        payload = {"workspace_id": workspace_id, "job_id": job_id, "fingerprint": fingerprint,
                   "generated_at": datetime.now(timezone.utc).isoformat(), "question": question,
                   "include_latest_session": include_latest_session,
                   "analysis_diagnostics": diagnostics,
                   "verified_context": {
                       "git": evidence.get("git"),
                       "research_context": compact_research,
                       "indexed_experiments": compact_experiments,
                   },
                   "summary": summary}
        status, encoded, error = "ready", json.dumps(payload, ensure_ascii=False), None
    except subprocess.TimeoutExpired:
        status, encoded, error = "failed", None, json.dumps({
            "error_type": "timeout",
            "timeout_seconds": config.summarizer_timeout_seconds,
            "message": "Codex workspace summary exceeded the configured time limit.",
            "diagnostics": diagnostics,
        })
    except Exception as exc:
        status, encoded, error = "failed", None, json.dumps({
            "error_type": "worker_failure",
            "message": str(exc)[:500],
            "diagnostics": diagnostics,
        })
    now = datetime.now(timezone.utc).isoformat()
    progress = "completed" if status == "ready" else "failed"
    with _connect(config) as connection:
        connection.execute("UPDATE analysis_jobs SET status=?,summary_json=?,generated_at=?,updated_at=?,progress=?,error=? WHERE workspace_id=? AND job_id=?",
                           (status, encoded, now, now, progress, error, workspace_id, job_id))


def get_summary(config: ServerConfig, workspace_id: str, wait_seconds: int = 0) -> dict[str, Any]:
    _workspace(config, workspace_id)
    _ensure_tables(config)
    deadline = time.monotonic() + max(0, min(int(wait_seconds), 25))
    while True:
        with _connect(config) as connection:
            row = connection.execute(
                "SELECT * FROM analysis_jobs WHERE workspace_id=? ORDER BY updated_at DESC LIMIT 1", (workspace_id,),
            ).fetchone()
            if row is None:
                row = connection.execute("SELECT * FROM workspace_summaries WHERE workspace_id = ?", (workspace_id,)).fetchone()
        if row is None:
            return {"workspace_id": workspace_id, "status": "missing"}
        if row["status"] != "running" or time.monotonic() >= deadline:
            result = {"workspace_id": workspace_id, "status": row["status"], "job_id": row["job_id"],
                      "generated_at": row["generated_at"], "started_at": row["started_at"],
                      "updated_at": row["updated_at"], "progress": row["progress"]}
            if row["status"] == "ready":
                result.update(json.loads(row["summary_json"]))
            if row["status"] in {"failed", "interrupted"}:
                try:
                    result["error"] = json.loads(row["error"])
                except (TypeError, json.JSONDecodeError):
                    result["error"] = {"error_type": "legacy_failure", "message": row["error"]}
            return result
        time.sleep(0.5)


def request_analysis(
    config: ServerConfig, workspace_id: str, question: str = "",
    include_latest_session: bool = True, refresh: bool = False,
) -> dict[str, Any]:
    result = request_summary(
        config, workspace_id, force=refresh, question=question,
        include_latest_session=include_latest_session,
    )
    status = "completed" if result["status"] == "ready" else result["status"]
    return {**result, "status": status, "analysis_type": "codex_workspace_analysis"}


def _analysis_result(payload: dict[str, Any]) -> dict[str, Any]:
    summary = payload.get("summary", {})
    return {
        "question": payload.get("question", ""),
        "analysis_diagnostics": payload.get("analysis_diagnostics", {}),
        "verified_facts": payload.get("verified_context", {}),
        "latest_codex_working_context": summary.get("latest_codex_session"),
        "codex_analysis": {
            "objective": summary.get("objective"),
            "current_status": summary.get("current_status"),
            "open_questions": summary.get("open_questions", []),
            "suggested_next_actions": summary.get("suggested_next_actions", []),
        },
        "codex_fact_summary": summary.get("recent_experiment_facts", [])[:6],
        "evidence_refs": summary.get("evidence_refs", []),
    }


def _bounded_result(value: dict[str, Any], max_bytes: int) -> dict[str, Any]:
    def clip(item: Any, string_limit: int = 1200) -> Any:
        if isinstance(item, str):
            return item if len(item) <= string_limit else item[:string_limit] + "…"
        if isinstance(item, list):
            return [clip(child, string_limit) for child in item[:8]]
        if isinstance(item, dict):
            return {key: clip(child, string_limit) for key, child in item.items()}
        return item
    result = clip(value)
    if len(json.dumps(result, ensure_ascii=False).encode()) <= max_bytes:
        return result
    result["codex_fact_summary"] = []
    verified = result.get("verified_facts", {})
    if isinstance(verified, dict) and isinstance(verified.get("indexed_experiments"), list):
        verified["indexed_experiments"] = verified["indexed_experiments"][:2]
    if len(json.dumps(result, ensure_ascii=False).encode()) <= max_bytes:
        result["response_truncated"] = True
        return result
    result = clip(result, 500)
    result["response_truncated"] = True
    return result


def get_job(config: ServerConfig, job_id: str, wait_seconds: int = 0) -> dict[str, Any]:
    if not re.fullmatch(r"[A-Za-z0-9_.-]{3,100}", job_id):
        raise ValueError("invalid job_id")
    _ensure_tables(config)
    deadline = time.monotonic() + max(0, min(int(wait_seconds), 20))
    while True:
        with _connect(config) as connection:
            row = connection.execute(
                "SELECT * FROM analysis_jobs WHERE job_id=?", (job_id,),
            ).fetchone()
            if row is None:
                row = connection.execute(
                    "SELECT * FROM workspace_summaries WHERE job_id=?", (job_id,),
                ).fetchone()
        if row is None:
            return {"job_id": job_id, "status": "not_found"}
        if row["status"] != "running" or time.monotonic() >= deadline:
            status = "completed" if row["status"] == "ready" else row["status"]
            result: dict[str, Any] = {
                "job_id": job_id, "workspace_id": row["workspace_id"], "status": status,
                "progress": row["progress"], "started_at": row["started_at"],
                "updated_at": row["updated_at"], "generated_at": row["generated_at"],
            }
            if row["status"] == "ready":
                payload = json.loads(row["summary_json"])
                result["result"] = _bounded_result(_analysis_result(payload), config.project.max_response_bytes)
            elif row["status"] in {"failed", "interrupted"}:
                try:
                    result["error"] = json.loads(row["error"])
                except (TypeError, json.JSONDecodeError):
                    result["error"] = {"error_type": "legacy_failure", "message": row["error"]}
            return result
        time.sleep(0.5)
