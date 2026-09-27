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


TEXT_SUFFIXES = {
    ".c", ".cc", ".cpp", ".csv", ".go", ".java", ".json", ".jsonl", ".md",
    ".py", ".r", ".rs", ".sh", ".tex", ".toml", ".ts", ".tsx", ".txt",
    ".yaml", ".yml",
}
INSPECT_VIEWS = {"outline", "search", "lines", "json_pointer", "full_bounded"}
MAX_INSPECT_FILE_BYTES = 10_000_000
SENSITIVE_FILE_SUFFIXES = {".key", ".pem", ".p12", ".pfx"}
SENSITIVE_FILE_NAMES = {"id_rsa", "id_ed25519", "credentials.json", "secrets.json"}
DEFAULT_BLOCKED_PARTS = {
    ".git", ".env", ".venv", "__pycache__", "checkpoints", "datasets",
    "node_modules", "site-packages", "tmp", "wandb",
}
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
    if any(part in DEFAULT_BLOCKED_PARTS for part in relative_workspace.parts):
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
                if path in seen or not path.is_file() or path.suffix.lower() not in TEXT_SUFFIXES:
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


def register_evidence(
    config: ServerConfig, workspace: WorkspaceConfig, path: Path, kind: str,
    authority: str = "observed_artifact", start_line: int | None = None,
    end_line: int | None = None,
) -> dict[str, Any]:
    resolved = path.resolve()
    if _is_denied(config, workspace, resolved) or not resolved.is_file():
        raise ValueError("evidence path is outside the permitted workspace")
    raw = resolved.read_bytes()
    content_sha256 = hashlib.sha256(raw).hexdigest()
    relative = resolved.relative_to(workspace.root).as_posix()
    identity = f"{workspace.workspace_id}\0{relative}\0{start_line}\0{end_line}\0{content_sha256}"
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
    return {
        "evidence_ref": evidence_ref, "kind": kind, "path": relative,
        "location": {"start_line": start_line, "end_line": end_line},
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
    for path, asset in _candidate_files(config, workspace, kinds):
        if scanned >= 5000:
            break
        scanned += 1
        try:
            if path.stat().st_size > 2_000_000:
                continue
            lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        except OSError:
            continue
        relative = path.relative_to(workspace.root).as_posix()
        path_text = relative.casefold()
        for index, line in enumerate(lines, start=1):
            line_text = line.casefold()
            score = sum(3 for token in tokens if token in line_text) + sum(1 for token in tokens if token in path_text)
            if not score:
                continue
            start, end = max(1, index - 2), min(len(lines), index + 2)
            record = register_evidence(config, workspace, path, asset.kind, asset.authority, start, end)
            record["snippet"] = "\n".join(lines[start - 1:end])[:900]
            matches.append((score, path.stat().st_mtime, record))
            break
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
            "truncated": scanned >= 5000 or len(results) < min(limit, len(matches)),
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
            raw = path.read_bytes()
            current_sha = hashlib.sha256(raw).hexdigest()
            text = raw.decode("utf-8", errors="replace")
            if detail == "structured" and path.suffix.lower() in {".json", ".yaml", ".yml", ".toml"}:
                content = text[:budget]
            elif detail == "full_bounded":
                content = text[:budget]
            else:
                lines = text.splitlines()
                start = row["start_line"] or 1
                end = row["end_line"] or min(len(lines), start + 40)
                content = "\n".join(lines[max(0, start - 1):end])[:budget]
            budget -= len(content.encode("utf-8"))
            results.append({
                "evidence_ref": evidence_ref, "status": "ok" if current_sha == row["content_sha256"] else "stale_reference",
                "workspace_id": workspace.workspace_id, "kind": row["kind"],
                "authority": row["authority"], "path": row["relative_path"],
                "location": {"start_line": row["start_line"], "end_line": row["end_line"]},
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
    if resolved.suffix.casefold() not in TEXT_SUFFIXES:
        raise ValueError("inspect_file supports configured text formats only")
    if resolved.stat().st_size > MAX_INSPECT_FILE_BYTES:
        raise ValueError(f"file exceeds the {MAX_INSPECT_FILE_BYTES} byte inspection limit")
    return resolved


def _asset_for_path(workspace: WorkspaceConfig, path: Path) -> AssetConfig | None:
    return next((asset for asset in _inferred_assets(workspace) if _matches_asset(path, workspace, asset)), None)


def _json_outline(value: Any, depth: int = 0) -> Any:
    if depth >= 2:
        if isinstance(value, dict):
            return {"type": "object", "key_count": len(value)}
        if isinstance(value, list):
            return {"type": "array", "length": len(value)}
        return {"type": type(value).__name__}
    if isinstance(value, dict):
        return {
            "type": "object", "key_count": len(value),
            "keys": {
                str(key)[:160]: _json_outline(item, depth + 1)
                for key, item in list(value.items())[:40]
            },
            "keys_truncated": len(value) > 40,
        }
    if isinstance(value, list):
        sample = [_json_outline(item, depth + 1) for item in value[:3]]
        return {"type": "array", "length": len(value), "sample_shapes": sample}
    return {"type": type(value).__name__}


def _json_pointer_value(value: Any, pointer: str) -> Any:
    if pointer == "":
        return value
    if not pointer.startswith("/") or len(pointer) > 1000:
        raise ValueError("json_pointer must be empty or an RFC 6901 path beginning with /")
    current = value
    for raw_part in pointer[1:].split("/"):
        part = raw_part.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict):
            if part not in current:
                raise ValueError(f"json_pointer key not found: {part}")
            current = current[part]
        elif isinstance(current, list):
            try:
                index = int(part)
            except ValueError as exc:
                raise ValueError(f"json_pointer array index is invalid: {part}") from exc
            if index < 0 or index >= len(current):
                raise ValueError(f"json_pointer array index is out of range: {part}")
            current = current[index]
        else:
            raise ValueError(f"json_pointer cannot descend through {type(current).__name__}")
    return current


def inspect_file_data(
    config: ServerConfig, path: str, workspace_id: str | None = None,
    view: str = "outline", query: str | None = None,
    start_line: int | None = None, end_line: int | None = None,
    json_pointer: str | None = None, max_chars: int = 8000,
) -> dict[str, Any]:
    """Inspect one exact, safe text file without requiring it to be search-indexed."""
    workspace = resolve_workspace(config, workspace_id)
    view = str(view).strip().lower()
    if view not in INSPECT_VIEWS:
        raise ValueError(f"view must be one of: {', '.join(sorted(INSPECT_VIEWS))}")
    response_cap = max(500, config.project.max_response_bytes - 2500)
    max_chars = max(500, min(int(max_chars), 10_000, response_cap))
    resolved = _resolve_inspect_path(config, workspace, path)
    raw = resolved.read_bytes()
    text = raw.decode("utf-8", errors="replace")
    lines = text.splitlines()
    relative = resolved.relative_to(workspace.root).as_posix()
    asset = _asset_for_path(workspace, resolved)
    authority = asset.authority if asset else "observed_artifact"
    kind = asset.kind if asset else "workspace_file"
    base = {
        "resolved_workspace_id": workspace.workspace_id,
        "path": relative,
        "view": view,
        "file": {
            "suffix": resolved.suffix.casefold(), "size_bytes": len(raw),
            "line_count": len(lines),
            "modified_at": datetime.fromtimestamp(resolved.stat().st_mtime, timezone.utc).isoformat(),
            "sha256": hashlib.sha256(raw).hexdigest(),
        },
        "kind": kind, "authority": authority,
        "configured_asset": asset.asset_id if asset else None,
    }

    if view == "search":
        needle = str(query or "").strip()
        if not needle or len(needle) > 300:
            raise ValueError("query must contain 1 to 300 characters for search view")
        matches: list[dict[str, Any]] = []
        remaining = max_chars
        for index, line in enumerate(lines, start=1):
            if needle.casefold() not in line.casefold():
                continue
            first, last = max(1, index - 2), min(len(lines), index + 2)
            content = "\n".join(lines[first - 1:last])
            content = content[:remaining]
            if not content:
                break
            evidence = register_evidence(config, workspace, resolved, kind, authority, first, last)
            matches.append({
                "evidence_ref": evidence["evidence_ref"], "line": index,
                "location": {"start_line": first, "end_line": last}, "content": content,
            })
            remaining -= len(content)
            if len(matches) >= 12 or remaining <= 0:
                break
        return {
            **base, "query": needle, "matches": matches,
            "match_count_returned": len(matches),
            "truncated": len(matches) >= 12 or remaining <= 0,
        }

    if view == "lines":
        first = int(start_line or 1)
        last = int(end_line or min(len(lines), first + 79))
        if first < 1 or first > max(1, len(lines)) or last < first or last - first + 1 > 200:
            raise ValueError("lines view requires a valid range of at most 200 lines")
        last = min(last, len(lines))
        content = "\n".join(lines[first - 1:last])[:max_chars]
        evidence = register_evidence(config, workspace, resolved, kind, authority, first, last)
        return {
            **base, "location": {"start_line": first, "end_line": last},
            "content": content, "evidence_ref": evidence["evidence_ref"],
            "truncated": len(content) < len("\n".join(lines[first - 1:last])),
        }

    if view == "json_pointer":
        if resolved.suffix.casefold() != ".json":
            raise ValueError("json_pointer view requires a .json file")
        try:
            value = json.loads(text)
        except json.JSONDecodeError as exc:
            raise ValueError(f"file is not valid JSON: {exc}") from exc
        pointer = str(json_pointer if json_pointer is not None else "")
        selected = _json_pointer_value(value, pointer)
        content = json.dumps(selected, ensure_ascii=False, indent=2)
        evidence = register_evidence(config, workspace, resolved, kind, authority)
        return {
            **base, "json_pointer": pointer, "content": content[:max_chars],
            "selected_shape": _json_outline(selected), "evidence_ref": evidence["evidence_ref"],
            "truncated": len(content) > max_chars,
        }

    if view == "outline":
        suffix = resolved.suffix.casefold()
        outline: Any
        if suffix == ".json":
            try:
                outline = _json_outline(json.loads(text))
            except json.JSONDecodeError as exc:
                raise ValueError(f"file is not valid JSON: {exc}") from exc
        elif suffix == ".md":
            headings = [
                {"line": index, "level": len(match.group(1)), "title": match.group(2).strip()[:300]}
                for index, line in enumerate(lines, start=1)
                if (match := re.match(r"^(#{1,6})\s+(.+?)\s*$", line))
            ]
            outline = {"type": "markdown", "headings": headings[:60],
                       "headings_truncated": len(headings) > 60}
        else:
            symbols = [
                {"line": index, "text": line.strip()[:300]}
                for index, line in enumerate(lines, start=1)
                if re.match(r"^\s*(class|def|async\s+def|fn|struct|enum|interface|function)\s+", line)
            ]
            outline = {"type": "text", "symbols": symbols[:60],
                       "symbols_truncated": len(symbols) > 60}
        encoded = json.dumps(outline, ensure_ascii=False)
        evidence = register_evidence(config, workspace, resolved, kind, authority)
        return {
            **base, "outline": outline, "evidence_ref": evidence["evidence_ref"],
            "truncated": len(encoded) > max_chars,
        }

    content = text[:max_chars]
    returned_lines = min(len(lines), content.count("\n") + (1 if content else 0))
    evidence = register_evidence(
        config, workspace, resolved, kind, authority, 1, max(1, returned_lines),
    )
    return {
        **base, "location": {"start_line": 1, "end_line": returned_lines},
        "content": content, "evidence_ref": evidence["evidence_ref"],
        "truncated": len(text) > max_chars,
    }


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
