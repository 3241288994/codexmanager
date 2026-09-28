from __future__ import annotations

from datetime import datetime, timezone
import fnmatch
import hashlib
from itertools import chain
import json
from pathlib import Path
import re
import sqlite3
import subprocess
from typing import Any, Iterable

import yaml

from .config import AssetConfig, ServerConfig, WorkspaceConfig
from .index import _connect


from .file_types import (
    TEXT_SUFFIXES, INSPECT_VIEWS, MAX_INSPECT_FILE_BYTES,
    SENSITIVE_FILE_SUFFIXES, SENSITIVE_FILE_NAMES, DEFAULT_BLOCKED_PARTS,
    readable_file, is_sensitive_file,
)
from .file_reader import (
    bound_response, inspect_supported_file, search_segments, file_sha256, read_evidence_content,
    _json_outline, _json_pointer_value,
)

SCOPE_KIND_MAP = {
    "code": "source_code",
    "docs": "project_docs",
    "configs": "configuration",
    "research": "research_state",
    "runs": "experiment_run",
}


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def resolve_workspace(config: ServerConfig, workspace_id: str | None = None) -> WorkspaceConfig:
    resolved = workspace_id or config.default_workspace_id
    if resolved in config.workspaces:
        return config.workspaces[resolved]
    lowered = resolved.casefold()
    matches = [
        workspace for workspace in config.workspaces.values()
        if lowered == workspace.name.casefold()
        or any(lowered == alias.casefold() for alias in workspace.aliases)
    ]
    if len(matches) == 1:
        return matches[0]
    if matches:
        raise ValueError(f"ambiguous workspace: {workspace_id}")
    raise ValueError(f"unknown workspace_id: {workspace_id}")


def _git(root: Path, *args: str) -> str | None:
    try:
        result = subprocess.run(
            ["git", "-C", str(root), *args], capture_output=True, text=True,
            check=False, timeout=5,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    return result.stdout.strip() if result.returncode == 0 else None


def git_state(workspace: WorkspaceConfig) -> dict[str, Any]:
    commit = _git(workspace.root, "rev-parse", "--short", "HEAD")
    branch = _git(workspace.root, "branch", "--show-current")
    head_ref = _git(workspace.root, "symbolic-ref", "--quiet", "--short", "HEAD")
    dirty = _git(workspace.root, "status", "--porcelain")
    return {
        "branch": branch,
        "commit": commit,
        "head_state": "committed" if commit else "unborn" if head_ref else "unavailable",
        "dirty": bool(dirty),
        "changed_path_count": len(dirty.splitlines()) if dirty else 0,
    }


def _capabilities(workspace: WorkspaceConfig) -> list[str]:
    capabilities = {"evidence_search", "codex_analysis"}
    adapters = set(workspace.adapters)
    kinds = {asset.kind for asset in workspace.assets}
    if adapters & {"generic_experiments", "hvs_experiments", "mlflow", "wandb"} or "experiment_run" in kinds:
        capabilities.add("experiments")
    if adapters & {"research_dossier", "generic_research"} or "research_state" in kinds:
        capabilities.add("research_context")
    return sorted(capabilities)


def list_workspaces_data(config: ServerConfig) -> dict[str, Any]:
    return {
        "default_workspace_id": config.default_workspace_id,
        "workspaces": [
            {
                "workspace_id": workspace.workspace_id,
                "name": workspace.name,
                "aliases": list(workspace.aliases),
                "capabilities": _capabilities(workspace),
                "status": "ready" if workspace.root.is_dir() else "unavailable",
            }
            for workspace in config.workspaces.values()
        ],
    }


def _is_denied(config: ServerConfig, workspace: WorkspaceConfig, path: Path) -> bool:
    try:
        relative_workspace = path.resolve().relative_to(workspace.root)
    except (OSError, ValueError):
        return True
    if any(part.casefold() in DEFAULT_BLOCKED_PARTS for part in relative_workspace.parts) or is_sensitive_file(path):
        return True
    try:
        relative_project = path.resolve().relative_to(config.project.root).as_posix()
    except ValueError:
        relative_project = relative_workspace.as_posix()
    return any(
        fnmatch.fnmatch(relative_project, pattern)
        or fnmatch.fnmatch(relative_workspace.as_posix(), pattern)
        for pattern in config.project.denied_globs
    )


def _inferred_assets(workspace: WorkspaceConfig) -> tuple[AssetConfig, ...]:
    if workspace.assets:
        return workspace.assets
    candidates = (
        ("docs", "project_docs", ("README*", "docs/**/*", "paper/**/*")),
        ("code", "source_code", ("src/**/*", "scripts/**/*", "tests/**/*")),
        ("configs", "configuration", ("configs/**/*", "*.toml", "*.yaml", "*.yml")),
        ("research", "research_state", ("research/**/*",)),
        ("experiments", "experiment_run", ("runs/**/*", "outputs/**/*", "results/**/*")),
    )
    return tuple(
        AssetConfig(asset_id, kind, includes, (), "generic", "observed_artifact", "metadata", True)
        for asset_id, kind, includes in candidates
    )


def _matches_asset(path: Path, workspace: WorkspaceConfig, asset: AssetConfig) -> bool:
    relative = path.relative_to(workspace.root).as_posix()
    def matches(pattern: str) -> bool:
        variants = {pattern}
        if "/**/*" in pattern:
            variants.add(pattern.replace("/**/*", "/*"))
        return any(fnmatch.fnmatch(relative, item) or Path(relative).match(item) for item in variants)
    included = any(matches(pattern) for pattern in asset.include)
    excluded = any(matches(pattern) for pattern in asset.exclude)
    return included and not excluded


def _candidate_files(config: ServerConfig, workspace: WorkspaceConfig, kinds: set[str]) -> Iterable[tuple[Path, AssetConfig]]:
    seen: set[Path] = set()
    for asset in _inferred_assets(workspace):
        if kinds and asset.kind not in kinds:
            continue
        for pattern in asset.include:
            paths: Iterable[Path] = workspace.root.glob(pattern)
            if "/**/*" in pattern:
                base = workspace.root / pattern.split("/**/*", 1)[0]
                if base.is_dir():
                    paths = chain(paths, base.rglob("*"))
            for path in paths:
                if path in seen or not path.is_file() or not readable_file(path):
                    continue
                if _is_denied(config, workspace, path) or not _matches_asset(path, workspace, asset):
                    continue
                seen.add(path)
                yield path, asset


def _ensure_evidence_table(config: ServerConfig) -> None:
    with _connect(config) as connection:
        connection.execute("""CREATE TABLE IF NOT EXISTS evidence_refs (
            evidence_ref TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, relative_path TEXT NOT NULL,
            kind TEXT NOT NULL, authority TEXT NOT NULL, start_line INTEGER, end_line INTEGER,
            content_sha256 TEXT NOT NULL, modified_at TEXT NOT NULL, created_at TEXT NOT NULL
        )""")
        connection.execute("""CREATE TABLE IF NOT EXISTS evidence_ref_locations (
            evidence_ref TEXT PRIMARY KEY, location_json TEXT NOT NULL
        )""")


def register_evidence(
    config: ServerConfig, workspace: WorkspaceConfig, path: Path, kind: str,
    authority: str = "observed_artifact", start_line: int | None = None,
    end_line: int | None = None, source_location: dict[str, int] | None = None,
) -> dict[str, Any]:
    resolved = path.resolve()
    if _is_denied(config, workspace, resolved) or not resolved.is_file():
        raise ValueError("evidence path is outside the permitted workspace")
    if resolved.stat().st_size > MAX_INSPECT_FILE_BYTES:
        raise ValueError("Evidence file exceeds the inspection limit")
    content_sha256 = file_sha256(resolved)
    relative = resolved.relative_to(workspace.root).as_posix()
    identity = f"{workspace.workspace_id}\0{relative}\0{start_line}\0{end_line}\0{content_sha256}"
    if source_location:
        identity += "\0" + json.dumps(source_location, sort_keys=True)
    evidence_ref = "ev_" + hashlib.sha256(identity.encode()).hexdigest()[:24]
    modified_at = datetime.fromtimestamp(resolved.stat().st_mtime, timezone.utc).isoformat()
    _ensure_evidence_table(config)
    with _connect(config) as connection:
        connection.execute(
            """INSERT OR REPLACE INTO evidence_refs
               (evidence_ref,workspace_id,relative_path,kind,authority,start_line,end_line,content_sha256,modified_at,created_at)
               VALUES(?,?,?,?,?,?,?,?,?,?)""",
            (evidence_ref, workspace.workspace_id, relative, kind, authority, start_line,
             end_line, content_sha256, modified_at, utc_now()),
        )
        if source_location:
            connection.execute(
                "INSERT OR REPLACE INTO evidence_ref_locations VALUES (?,?)",
                (evidence_ref, json.dumps(source_location, sort_keys=True)),
            )
    return {
        "evidence_ref": evidence_ref, "kind": kind, "path": relative,
        "location": source_location or {"start_line": start_line, "end_line": end_line},
        "sha256": content_sha256, "modified_at": modified_at, "authority": authority,
    }


def search_evidence_data(
    config: ServerConfig, query: str, workspace_id: str | None = None,
    scopes: list[str] | None = None, limit: int = 8,
) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    query = query.strip()
    if not query or len(query) > 300:
        raise ValueError("query must contain 1 to 300 characters")
    limit = max(1, min(int(limit), 20))
    kinds = {SCOPE_KIND_MAP.get(scope, scope) for scope in (scopes or [])}
    tokens = [token.casefold() for token in re.findall(r"[\w.-]+", query) if len(token) > 1]
    if not tokens:
        tokens = [query.casefold()]
    matches: list[tuple[int, float, dict[str, Any]]] = []
    scanned = 0
    skipped_size = 0
    skipped_unreadable = 0
    extraction_truncated = 0
    for path, asset in _candidate_files(config, workspace, kinds):
        if scanned >= 5000:
            break
        scanned += 1
        try:
            if path.stat().st_size > 2_000_000:
                skipped_size += 1
                continue
            relative = path.relative_to(workspace.root).as_posix()
            path_text = relative.casefold()
            found = False
            for text, source, clipped in search_segments(path):
                extraction_truncated += int(clipped)
                lines = text.splitlines()
                for index, line in enumerate(lines, start=1):
                    score = sum(3 for token in tokens if token in line.casefold()) + sum(1 for token in tokens if token in path_text)
                    if not score:
                        continue
                    first, last = max(1, index - 2), min(len(lines), index + 2)
                    location = {**source, "start_line": first, "end_line": last}
                    record = register_evidence(
                        config, workspace, path, asset.kind, asset.authority,
                        first, last, source_location=location,
                    )
                    record["snippet"] = "\n".join(lines[first - 1:last])[:900]
                    matches.append((score, path.stat().st_mtime, record))
                    found = True
                    break
                if found:
                    break
        except (OSError, ValueError, UnicodeError):
            skipped_unreadable += 1
    matches.sort(key=lambda item: (item[0], item[1]), reverse=True)
    results: list[dict[str, Any]] = []
    remaining = max(1000, config.project.max_response_bytes - 1200)
    for _, _, record in matches[:limit]:
        size = len(json.dumps(record, ensure_ascii=False).encode())
        if size > remaining:
            break
        results.append(record)
        remaining -= size
    return {
        "resolved_workspace_id": workspace.workspace_id,
        "query": query,
        "results": results,
        "coverage": {
            "files_scanned": scanned,
            "files_skipped_size": skipped_size,
            "files_skipped_unreadable": skipped_unreadable,
            "extractions_truncated": extraction_truncated,
            "truncated": scanned >= 5000 or extraction_truncated > 0 or len(results) < min(limit, len(matches)),
        },
    }


def get_evidence_data(
    config: ServerConfig, evidence_refs: list[str], workspace_id: str | None = None,
    detail: str = "excerpt",
) -> dict[str, Any]:
    if detail not in {"excerpt", "structured", "full_bounded"}:
        raise ValueError("detail must be excerpt, structured, or full_bounded")
    if not evidence_refs or len(evidence_refs) > 8:
        raise ValueError("provide 1 to 8 evidence_refs")
    requested_workspace = resolve_workspace(config, workspace_id) if workspace_id else None
    _ensure_evidence_table(config)
    results: list[dict[str, Any]] = []
    budget = max(1000, config.project.max_response_bytes - 2000)
    with _connect(config) as connection:
        for evidence_ref in evidence_refs:
            row = connection.execute("SELECT * FROM evidence_refs WHERE evidence_ref=?", (evidence_ref,)).fetchone()
            if row is None:
                results.append({"evidence_ref": evidence_ref, "status": "not_found"})
                continue
            workspace = resolve_workspace(config, row["workspace_id"])
            if requested_workspace and requested_workspace.workspace_id != workspace.workspace_id:
                results.append({"evidence_ref": evidence_ref, "status": "workspace_mismatch"})
                continue
            path = (workspace.root / row["relative_path"]).resolve()
            if _is_denied(config, workspace, path) or not path.is_file():
                results.append({"evidence_ref": evidence_ref, "status": "unavailable"})
                continue
            location_row = connection.execute(
                "SELECT location_json FROM evidence_ref_locations WHERE evidence_ref=?", (evidence_ref,),
            ).fetchone()
            location = json.loads(location_row["location_json"]) if location_row else {
                "start_line": row["start_line"], "end_line": row["end_line"],
            }
            if path.stat().st_size > MAX_INSPECT_FILE_BYTES:
                results.append({"evidence_ref": evidence_ref, "status": "unavailable"})
                continue
            current_sha = file_sha256(path)
            status = "ok" if current_sha == row["content_sha256"] else "stale_reference"
            content = ""
            if status == "ok":
                try:
                    content = read_evidence_content(path, location if detail == "excerpt" else {}, budget)
                except (OSError, ValueError):
                    status = "unavailable"
            content = content.encode("utf-8")[:budget].decode("utf-8", errors="ignore")
            budget -= len(content.encode("utf-8"))
            results.append({
                "evidence_ref": evidence_ref, "status": status,
                "workspace_id": workspace.workspace_id, "kind": row["kind"],
                "authority": row["authority"], "path": row["relative_path"],
                "location": location,
                "indexed_sha256": row["content_sha256"], "current_sha256": current_sha,
                "content": content,
            })
            if budget <= 0:
                break
    return {
        "resolved_workspace_id": requested_workspace.workspace_id if requested_workspace else None,
        "detail": detail, "results": results,
    }


def _resolve_inspect_path(
    config: ServerConfig, workspace: WorkspaceConfig, requested_path: str,
) -> Path:
    value = str(requested_path).strip()
    if not value or len(value) > 2000 or "\x00" in value:
        raise ValueError("path must contain 1 to 2000 characters")
    candidate = Path(value).expanduser()
    resolved = (candidate if candidate.is_absolute() else workspace.root / candidate).resolve()
    if not resolved.is_relative_to(workspace.root):
        raise ValueError("file path must remain inside the selected workspace")
    if _is_denied(config, workspace, resolved):
        raise ValueError("file is denied by workspace policy")
    if not resolved.is_file():
        raise ValueError("file does not exist in the selected workspace")
    name = resolved.name.casefold()
    if name.startswith(".env") or name in SENSITIVE_FILE_NAMES or resolved.suffix.casefold() in SENSITIVE_FILE_SUFFIXES:
        raise ValueError("sensitive credential files cannot be inspected")
    if not readable_file(resolved):
        raise ValueError("inspect_file does not support this file format")
    if resolved.stat().st_size > MAX_INSPECT_FILE_BYTES:
        raise ValueError(f"file exceeds the {MAX_INSPECT_FILE_BYTES} byte inspection limit")
    return resolved


def _asset_for_path(workspace: WorkspaceConfig, path: Path) -> AssetConfig | None:
    return next((asset for asset in _inferred_assets(workspace) if _matches_asset(path, workspace, asset)), None)


def inspect_file_data(
    config: ServerConfig, path: str, workspace_id: str | None = None,
    view: str = "outline", query: str | None = None,
    start_line: int | None = None, end_line: int | None = None,
    json_pointer: str | None = None, max_chars: int = 8000,
    start_page: int | None = None, end_page: int | None = None,
) -> dict[str, Any]:
    """Inspect a supported file through the same parser as direct-path reads."""
    workspace = resolve_workspace(config, workspace_id)
    resolved = _resolve_inspect_path(config, workspace, path)
    response_cap = max(500, config.project.max_response_bytes - 2500)
    result = inspect_supported_file(
        resolved, view, query, start_line, end_line, start_page, end_page,
        json_pointer, min(max_chars, response_cap),
    )
    asset = _asset_for_path(workspace, resolved)
    authority = asset.authority if asset else "observed_artifact"
    kind = asset.kind if asset else "workspace_file"
    def evidence(location):
        location = dict(location or {})
        return register_evidence(
            config, workspace, resolved, kind, authority,
            location.get("start_line"), location.get("end_line"),
            source_location=location,
        )["evidence_ref"]
    if result["view"] == "search":
        for match in result.get("matches", []):
            location = match.get("location") or (
                {"start_page": match["page"], "end_page": match["page"]} if "page" in match else {}
            )
            match["evidence_ref"] = evidence(location)
    else:
        result["evidence_ref"] = evidence(result.get("location"))
    result["file"]["modified_at"] = datetime.fromtimestamp(resolved.stat().st_mtime, timezone.utc).isoformat()
    return bound_response({
        "resolved_workspace_id": workspace.workspace_id,
        "path": resolved.relative_to(workspace.root).as_posix(),
        "kind": kind, "authority": authority,
        "configured_asset": asset.asset_id if asset else None, **result,
    }, config.project.max_response_bytes)


def _read_json(path: Path, max_bytes: int = 1_000_000) -> dict[str, Any] | None:
    try:
        if path.stat().st_size > max_bytes:
            return None
        value = json.loads(path.read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else None
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return None


def _latest_research_root(workspace: WorkspaceConfig) -> Path | None:
    candidates: list[tuple[int, float, Path]] = []
    for path in workspace.root.glob("research/idea_runs/*/run_state.json"):
        state = _read_json(path, 200_000)
        if state is None:
            continue
        candidates.append((1 if state.get("status") == "active" else 0, path.stat().st_mtime, path.parent))
    return max(candidates, default=(0, 0.0, None), key=lambda item: (item[0], item[1]))[2]


def research_context_data(
    config: ServerConfig, workspace_id: str | None = None, research_id: str = "active",
    sections: list[str] | None = None,
) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    wanted = set(sections or ["objective", "hypotheses", "claims", "decisions", "open_questions"])
    map_payload: dict[str, Any] = {}
    if wanted & {"map_focus", "active_branches"}:
        from .research_map import focus_capsule, load_research_map
        map_value = load_research_map(config, workspace.workspace_id)
        if "map_focus" in wanted:
            map_payload["map_focus"] = focus_capsule(map_value)
        if "active_branches" in wanted:
            map_payload["active_branches"] = [
                {
                    "id": node["id"], "title": node["title"], "status": node["status"],
                    "summary": node["summary"][:500],
                    "authority": node["authority"], "source_refs": node["source_refs"][:8],
                }
                for node in map_value["nodes"]
                if node["type"] == "branch" and node["status"] != "archived"
            ][:12]
    if research_id == "active":
        research_root = _latest_research_root(workspace)
    else:
        candidate = (workspace.root / "research" / "idea_runs" / research_id).resolve()
        research_root = candidate if candidate.is_dir() and candidate.is_relative_to(workspace.root) else None
    if research_root is None:
        return {
            "resolved_workspace_id": workspace.workspace_id, "research_id": research_id,
            "status": "unavailable", "coverage": "unstructured",
            "missing": sorted(wanted - {"map_focus", "active_branches"}),
            "recommended_action": "request workspace onboarding analysis", **map_payload,
        }
    state = _read_json(research_root / "run_state.json", 300_000) or {}
    contract = _read_json(research_root / "research_contract.json") or {}
    claims_data = _read_json(research_root / "claim_evidence.json") or {}
    decision = _read_json(research_root / "decision.json", 300_000) or {}
    payload: dict[str, Any] = {
        "resolved_workspace_id": workspace.workspace_id,
        "research_id": state.get("run_id", research_root.name),
        "status": state.get("status", "unknown"), "phase": state.get("phase"),
        "updated_at": state.get("updated_at"), "authority": "canonical_fact",
    }
    payload.update(map_payload)
    def bounded_text(value: Any, limit: int = 700) -> Any:
        if not isinstance(value, str):
            return value
        return value if len(value) <= limit else value[:limit] + "…"
    if "objective" in wanted:
        payload["objective"] = contract.get("goal")
        boundary = contract.get("problem_boundary")
        if isinstance(boundary, dict):
            payload["problem_boundary"] = {
                "decision_to_make": bounded_text(boundary.get("decision_to_make"), 1000),
                "in_scope": [bounded_text(item, 400) for item in boundary.get("in_scope", [])[:6]],
                "out_of_scope": [bounded_text(item, 400) for item in boundary.get("out_of_scope", [])[:6]],
            }
        else:
            payload["problem_boundary"] = boundary
    if "hypotheses" in wanted:
        hypothesis = contract.get("claim")
        payload["hypotheses"] = [{
            key: bounded_text(hypothesis.get(key), 900)
            for key in ("claim_id", "one_sentence", "falsifier", "freeze_status", "frozen_at")
            if key in hypothesis
        }] if isinstance(hypothesis, dict) else []
    if "claims" in wanted:
        payload["claims"] = [
            {
                key: bounded_text(claim.get(key))
                for key in ("claim_id", "text", "falsifier", "status", "evidence_ids", "alternative_explanation_ids")
                if key in claim
            }
            for claim in (claims_data.get("claims") or [])[:8]
            if isinstance(claim, dict)
        ]
    if "evidence" in wanted:
        payload["evidence"] = [
            {
                "evidence_id": item.get("evidence_id"), "kind": item.get("kind"),
                "provenance_summary": bounded_text(item.get("provenance_summary"), 700),
                "supports_claim_ids": item.get("supports_claim_ids", []),
                "contradicts_claim_ids": item.get("contradicts_claim_ids", []),
                "limitations": [bounded_text(limit, 300) for limit in item.get("limitations", [])[:3]],
                "verification_status": item.get("verification_status"),
            }
            for item in (claims_data.get("evidence") or [])[:8]
            if isinstance(item, dict)
        ]
        payload["causal_chain"] = [
            {
                "order": item.get("order"), "node": bounded_text(item.get("node"), 500),
                "status": item.get("status"), "evidence_ids": item.get("evidence_ids", []),
            }
            for item in (claims_data.get("causal_chain") or [])[:10]
            if isinstance(item, dict)
        ]
    if "decisions" in wanted:
        payload["decision"] = decision
    if "open_questions" in wanted:
        payload["open_questions"] = {
            "pending_decision": bounded_text(state.get("pending_decision"), 1600),
            "unresolved_risks": [bounded_text(item, 400) for item in decision.get("unresolved_risks", [])[:8]],
            "next_evidence_tier": bounded_text(decision.get("next_evidence_tier"), 700),
        }
    refs = []
    for filename in ("run_state.json", "research_contract.json", "claim_evidence.json", "decision.json"):
        path = research_root / filename
        if path.is_file():
            refs.append(register_evidence(config, workspace, path, "research_state", "canonical_fact")["evidence_ref"])
    payload["evidence_refs"] = refs
    payload["coverage"] = "structured_adapter"
    return payload


def _safe_context(workspace: WorkspaceConfig, max_bytes: int) -> dict[str, Any] | None:
    if not workspace.context_file.is_file():
        return None
    try:
        raw = workspace.context_file.read_bytes()[:max_bytes]
        value = yaml.safe_load(raw.decode("utf-8"))
        return value if isinstance(value, dict) else None
    except (OSError, UnicodeDecodeError, yaml.YAMLError):
        return None


def workspace_overview_data(config: ServerConfig, workspace_id: str | None = None) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    assets = _inferred_assets(workspace)
    asset_summary = [
        {
            "asset_id": asset.asset_id, "kind": asset.kind, "adapter": asset.adapter,
            "authority": asset.authority, "index_content": asset.index_content,
        }
        for asset in assets
    ]
    research = research_context_data(config, workspace.workspace_id, sections=["open_questions"])
    from .index import query_experiments
    from .summarizer import _latest_session, _recent_sessions
    experiments = query_experiments(config, workspace, detail="summary", limit=3)
    recent_experiments = [
        {
            "experiment_id": record["experiment_id"], "title": record["title"],
            "source_modified_at": record["source_modified_at"],
            "metrics": dict(list(record["metrics"].items())[:6]),
            "evidence": record["evidence"],
        }
        for record in experiments["experiments"]
    ]
    session = _latest_session(config, workspace)
    working_context = _recent_sessions(config, workspace)
    from .research_map import focus_capsule, load_research_map
    map_focus = focus_capsule(load_research_map(config, workspace.workspace_id))
    session_metadata = {
        key: value for key, value in session.items()
        if key in {"status", "reason", "session_id", "updated_at", "workspace_match", "message_count", "excerpt_sha256"}
    }
    with _connect(config) as connection:
        try:
            job = connection.execute(
                "SELECT job_id,status,progress,updated_at FROM analysis_jobs WHERE workspace_id=? ORDER BY updated_at DESC LIMIT 1",
                (workspace.workspace_id,),
            ).fetchone()
        except sqlite3.OperationalError:
            job = None
    return {
        "resolved_workspace_id": workspace.workspace_id,
        "workspace": {"name": workspace.name, "aliases": list(workspace.aliases)},
        "generated_at": utc_now(), "git": git_state(workspace),
        "capabilities": _capabilities(workspace), "assets": asset_summary,
        "reviewed_context": _safe_context(workspace, config.summarizer_max_context_bytes),
        "active_research": {
            key: research.get(key) for key in ("research_id", "status", "phase", "updated_at", "open_questions")
            if key in research
        },
        "recent_experiments": recent_experiments,
        "latest_codex_session": session_metadata,
        "working_context": working_context,
        "focus_capsule": map_focus,
        "analysis_job": dict(job) if job else None,
        "freshness": {"generated_at": utc_now(), "mode": "live_metadata_and_adapter_projection"},
        "coverage": {
            "research": research.get("coverage"),
            "experiment_index": experiments["index_refresh"],
            "limitations": [] if research.get("coverage") == "structured_adapter" else ["No structured research-state adapter matched"],
        },
    }
