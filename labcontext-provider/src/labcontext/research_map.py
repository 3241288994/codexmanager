from __future__ import annotations

from copy import deepcopy
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import sqlite3
import subprocess
import tempfile
import threading
from typing import Any

from .config import ServerConfig, WorkspaceConfig
from .context import resolve_workspace
from .index import _connect


SCHEMA_VERSION = 1
NODE_TYPES = {
    "core_idea", "claim", "branch", "current_target", "experiment",
    "evidence", "decision", "risk",
}
EDGE_RELATIONS = {
    "decomposes_into", "tests", "supports", "contradicts", "motivated_by",
    "blocked_by", "supersedes", "derived_from",
}
AUTHORITIES = {
    "user_reviewed", "canonical_fact", "observed_artifact", "working_hypothesis",
    "codex_inference", "unverified_session_digest", "provisional",
}
ORIGINS = {
    "manual", "structured_adapter", "experiment_index", "codex_suggestion",
    "codex_initialization", "session_digest", "context_file", "automatic",
}
PATCH_OPERATIONS = {
    "add_node", "update_node", "set_status", "archive_node", "add_edge",
    "remove_edge", "attach_evidence", "set_current_focus",
}
MAP_PATCH_SCHEMA = {
    "type": "object",
    "additionalProperties": False,
    "required": ["base_revision", "summary", "operations"],
    "properties": {
        "base_revision": {"type": "integer", "minimum": 0},
        "summary": {"type": "string", "maxLength": 500},
        "operations": {
            "type": "array", "minItems": 1, "maxItems": 100,
            "items": {
                "type": "object",
                "additionalProperties": True,
                "required": ["op"],
                "properties": {"op": {"type": "string", "enum": sorted(PATCH_OPERATIONS)}},
            },
        },
    },
}
PROTECTED_CODEX_FIELDS = {"authority", "locked_fields", "origin"}
_LOCKS: dict[str, threading.RLock] = {}
_LOCKS_GUARD = threading.Lock()


class ResearchMapError(ValueError):
    pass


class ResearchMapConflict(ResearchMapError):
    pass


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def _lock(workspace: WorkspaceConfig) -> threading.RLock:
    key = str(workspace.root)
    with _LOCKS_GUARD:
        return _LOCKS.setdefault(key, threading.RLock())


def map_paths(workspace: WorkspaceConfig) -> dict[str, Path]:
    root = workspace.root / ".labcontext"
    return {
        "root": root,
        "map": root / "research-map.json",
        "layout": root / "research-map.layout.json",
        "events": root / "research-map.events.jsonl",
    }


def _clean_text(value: Any, field: str, limit: int, *, required: bool = False) -> str:
    text = " ".join(str(value or "").split()).strip()
    if required and not text:
        raise ResearchMapError(f"{field} is required")
    if len(text) > limit:
        raise ResearchMapError(f"{field} exceeds {limit} characters")
    return text


def _clean_id(value: Any, field: str) -> str:
    identifier = str(value or "").strip()
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}", identifier):
        raise ResearchMapError(f"invalid {field}")
    return identifier


def _bounded_strings(value: Any, field: str, limit: int = 24) -> list[str]:
    if value is None:
        return []
    if not isinstance(value, list) or len(value) > limit:
        raise ResearchMapError(f"{field} must be a list with at most {limit} items")
    return [_clean_text(item, field, 300, required=True) for item in value]


def _normalize_node(node: Any, *, existing: dict[str, Any] | None = None) -> dict[str, Any]:
    if not isinstance(node, dict):
        raise ResearchMapError("node must be an object")
    node_id = _clean_id(node.get("id"), "node id")
    node_type = str(node.get("type") or "").strip()
    if node_type not in NODE_TYPES:
        raise ResearchMapError(f"unsupported node type: {node_type}")
    authority = str(node.get("authority") or "working_hypothesis")
    origin = str(node.get("origin") or "manual")
    if authority not in AUTHORITIES:
        raise ResearchMapError(f"unsupported authority: {authority}")
    if origin not in ORIGINS:
        raise ResearchMapError(f"unsupported origin: {origin}")
    now = utc_now()
    created_at = str((existing or {}).get("created_at") or node.get("created_at") or now)
    metadata = node.get("metadata") or {}
    if not isinstance(metadata, dict):
        raise ResearchMapError("node metadata must be an object")
    if len(json.dumps(metadata, ensure_ascii=False).encode()) > 8_000:
        raise ResearchMapError("node metadata is too large")
    return {
        "id": node_id,
        "type": node_type,
        "title": _clean_text(node.get("title"), "node title", 240, required=True),
        "summary": _clean_text(node.get("summary"), "node summary", 1600),
        "status": _clean_text(node.get("status") or "active", "node status", 64, required=True),
        "authority": authority,
        "origin": origin,
        "locked_fields": sorted(set(_bounded_strings(node.get("locked_fields"), "locked_fields", 16))),
        "source_refs": sorted(set(_bounded_strings(node.get("source_refs"), "source_refs", 32))),
        "metadata": metadata,
        "created_at": created_at,
        "updated_at": str(node.get("updated_at") or (existing or {}).get("updated_at") or now),
    }


def _normalize_edge(edge: Any) -> dict[str, Any]:
    if not isinstance(edge, dict):
        raise ResearchMapError("edge must be an object")
    relation = str(edge.get("relation") or "").strip()
    if relation not in EDGE_RELATIONS:
        raise ResearchMapError(f"unsupported edge relation: {relation}")
    source = _clean_id(edge.get("source"), "edge source")
    target = _clean_id(edge.get("target"), "edge target")
    if source == target:
        raise ResearchMapError("self-referencing edges are not allowed")
    edge_id = str(edge.get("id") or f"edge:{source}:{relation}:{target}")
    origin = str(edge.get("origin") or "manual")
    if origin not in ORIGINS:
        raise ResearchMapError(f"unsupported edge origin: {origin}")
    return {
        "id": _clean_id(edge_id, "edge id"),
        "source": source,
        "target": target,
        "relation": relation,
        "label": _clean_text(edge.get("label"), "edge label", 120),
        "origin": origin,
        "source_refs": sorted(set(_bounded_strings(edge.get("source_refs"), "source_refs", 32))),
        "created_at": str(edge.get("created_at") or utc_now()),
    }


def empty_research_map(workspace: WorkspaceConfig) -> dict[str, Any]:
    now = utc_now()
    return {
        "schema_version": SCHEMA_VERSION,
        "workspace_id": workspace.workspace_id,
        "revision": 0,
        "review_state": "empty",
        "current_focus_node_id": None,
        "nodes": [],
        "edges": [],
        "created_at": now,
        "updated_at": now,
        "updated_by": "system",
    }


def validate_research_map(value: Any, workspace: WorkspaceConfig) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ResearchMapError("research map must be an object")
    if int(value.get("schema_version", 0)) != SCHEMA_VERSION:
        raise ResearchMapError("unsupported research map schema_version")
    if value.get("workspace_id") != workspace.workspace_id:
        raise ResearchMapError("research map workspace_id mismatch")
    raw_nodes = value.get("nodes") or []
    raw_edges = value.get("edges") or []
    if not isinstance(raw_nodes, list) or len(raw_nodes) > 500:
        raise ResearchMapError("research map supports at most 500 nodes")
    if not isinstance(raw_edges, list) or len(raw_edges) > 1500:
        raise ResearchMapError("research map supports at most 1500 edges")
    nodes = [_normalize_node(node, existing=node) for node in raw_nodes]
    node_ids = [node["id"] for node in nodes]
    if len(node_ids) != len(set(node_ids)):
        raise ResearchMapError("duplicate node id")
    edges = [_normalize_edge(edge) for edge in raw_edges]
    edge_ids = [edge["id"] for edge in edges]
    if len(edge_ids) != len(set(edge_ids)):
        raise ResearchMapError("duplicate edge id")
    known = set(node_ids)
    for edge in edges:
        if edge["source"] not in known or edge["target"] not in known:
            raise ResearchMapError(f"edge references an unknown node: {edge['id']}")
    graph: dict[str, list[str]] = {node_id: [] for node_id in known}
    for edge in edges:
        if edge["relation"] in {"decomposes_into", "supersedes"}:
            graph[edge["source"]].append(edge["target"])
    visiting: set[str] = set()
    visited: set[str] = set()
    def visit(node_id: str) -> None:
        if node_id in visiting:
            raise ResearchMapError("hierarchical research-map relations must be acyclic")
        if node_id in visited:
            return
        visiting.add(node_id)
        for target in graph[node_id]:
            visit(target)
        visiting.remove(node_id)
        visited.add(node_id)
    for node_id in graph:
        visit(node_id)
    focus = value.get("current_focus_node_id")
    if focus is not None and focus not in known:
        raise ResearchMapError("current_focus_node_id references an unknown node")
    return {
        "schema_version": SCHEMA_VERSION,
        "workspace_id": workspace.workspace_id,
        "revision": max(0, int(value.get("revision", 0))),
        "review_state": _clean_text(value.get("review_state") or "needs_review", "review_state", 64),
        "current_focus_node_id": focus,
        "nodes": nodes,
        "edges": edges,
        "created_at": str(value.get("created_at") or utc_now()),
        "updated_at": str(value.get("updated_at") or utc_now()),
        "updated_by": _clean_text(value.get("updated_by") or "unknown", "updated_by", 120),
    }


def _atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
    rendered = json.dumps(value, ensure_ascii=False, indent=2, sort_keys=False) + "\n"
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".tmp", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            handle.write(rendered)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def load_research_map(
    config: ServerConfig, workspace_id: str | None = None, *, create: bool = False,
) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    path = map_paths(workspace)["map"]
    with _lock(workspace):
        if not path.is_file():
            value = empty_research_map(workspace)
            if create:
                _atomic_json(path, value)
            return value
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise ResearchMapError(f"research map could not be read: {error}") from error
        return validate_research_map(value, workspace)


def _append_event(workspace: WorkspaceConfig, event: dict[str, Any]) -> None:
    path = map_paths(workspace)["events"]
    path.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(event, ensure_ascii=False, sort_keys=True) + "\n")
        handle.flush()
        os.fsync(handle.fileno())


def write_research_map(
    config: ServerConfig, workspace_id: str, value: dict[str, Any], *, actor: str,
    expected_revision: int | None, event_type: str, event_summary: str,
) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    with _lock(workspace):
        current = load_research_map(config, workspace.workspace_id)
        if expected_revision is not None and current["revision"] != expected_revision:
            raise ResearchMapConflict(
                f"revision_conflict: expected {expected_revision}, current {current['revision']}"
            )
        candidate = deepcopy(value)
        candidate.update({
            "schema_version": SCHEMA_VERSION,
            "workspace_id": workspace.workspace_id,
            "revision": current["revision"] + 1,
            "created_at": current.get("created_at") or utc_now(),
            "updated_at": utc_now(),
            "updated_by": actor[:120],
        })
        normalized = validate_research_map(candidate, workspace)
        _atomic_json(map_paths(workspace)["map"], normalized)
        _append_event(workspace, {
            "event_id": "mapevt_" + hashlib.sha256(
                f"{workspace.workspace_id}\0{normalized['revision']}\0{normalized['updated_at']}".encode()
            ).hexdigest()[:20],
            "event_type": event_type,
            "workspace_id": workspace.workspace_id,
            "revision": normalized["revision"],
            "actor": actor[:120],
            "summary": _clean_text(event_summary, "event summary", 500),
            "timestamp": normalized["updated_at"],
        })
        return normalized


def _node_index(value: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {node["id"]: node for node in value["nodes"]}


def _edge_index(value: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {edge["id"]: edge for edge in value["edges"]}


def apply_map_patch(
    config: ServerConfig, workspace_id: str, patch: dict[str, Any], *, actor: str = "user",
    proposal_source: str | None = None,
) -> dict[str, Any]:
    if not isinstance(patch, dict):
        raise ResearchMapError("patch must be an object")
    operations = patch.get("operations")
    if not isinstance(operations, list) or not operations or len(operations) > 100:
        raise ResearchMapError("patch operations must contain 1 to 100 items")
    base_revision = int(patch.get("base_revision", -1))
    workspace = resolve_workspace(config, workspace_id)
    with _lock(workspace):
        current = load_research_map(config, workspace.workspace_id)
        if current["revision"] != base_revision:
            raise ResearchMapConflict(
                f"revision_conflict: expected {base_revision}, current {current['revision']}"
            )
        candidate = deepcopy(current)
        nodes = _node_index(candidate)
        edges = _edge_index(candidate)
        codex_origin = bool(proposal_source) or actor.startswith("codex")
        for index, operation in enumerate(operations):
            if not isinstance(operation, dict):
                raise ResearchMapError(f"operation {index} must be an object")
            op = str(operation.get("op") or "")
            if op not in PATCH_OPERATIONS:
                raise ResearchMapError(f"unsupported patch operation: {op}")
            if op == "add_node":
                raw_node = deepcopy(operation.get("node"))
                if not isinstance(raw_node, dict):
                    raise ResearchMapError("add_node requires node")
                if codex_origin:
                    raw_node.setdefault("origin", "codex_suggestion")
                    raw_node.setdefault("authority", "codex_inference")
                node = _normalize_node(raw_node)
                if node["id"] in nodes:
                    raise ResearchMapError(f"node already exists: {node['id']}")
                nodes[node["id"]] = node
            elif op == "update_node":
                node_id = _clean_id(operation.get("node_id"), "node id")
                if node_id not in nodes:
                    raise ResearchMapError(f"unknown node: {node_id}")
                changes = operation.get("changes")
                if not isinstance(changes, dict) or not changes:
                    raise ResearchMapError("update_node requires changes")
                allowed = {"title", "summary", "status", "authority", "origin", "locked_fields", "source_refs", "metadata"}
                if set(changes) - allowed:
                    raise ResearchMapError("update_node contains unsupported fields")
                locked = set(nodes[node_id].get("locked_fields", []))
                if codex_origin and (set(changes) & (locked | PROTECTED_CODEX_FIELDS)):
                    raise ResearchMapError(f"Codex proposal modifies protected fields on {node_id}")
                merged = {**nodes[node_id], **changes, "updated_at": utc_now()}
                nodes[node_id] = _normalize_node(merged, existing=nodes[node_id])
            elif op == "set_status":
                node_id = _clean_id(operation.get("node_id"), "node id")
                if node_id not in nodes:
                    raise ResearchMapError(f"unknown node: {node_id}")
                if codex_origin and "status" in set(nodes[node_id].get("locked_fields", [])):
                    raise ResearchMapError(f"Codex proposal modifies locked status on {node_id}")
                nodes[node_id]["status"] = _clean_text(operation.get("status"), "status", 64, required=True)
                nodes[node_id]["updated_at"] = utc_now()
            elif op == "archive_node":
                node_id = _clean_id(operation.get("node_id"), "node id")
                if node_id not in nodes:
                    raise ResearchMapError(f"unknown node: {node_id}")
                nodes[node_id]["status"] = "archived"
                nodes[node_id]["updated_at"] = utc_now()
                if candidate.get("current_focus_node_id") == node_id:
                    candidate["current_focus_node_id"] = None
            elif op == "add_edge":
                raw_edge = deepcopy(operation.get("edge"))
                if not isinstance(raw_edge, dict):
                    raise ResearchMapError("add_edge requires edge")
                if codex_origin:
                    raw_edge.setdefault("origin", "codex_suggestion")
                edge = _normalize_edge(raw_edge)
                if edge["id"] in edges:
                    raise ResearchMapError(f"edge already exists: {edge['id']}")
                edges[edge["id"]] = edge
            elif op == "remove_edge":
                edge_id = _clean_id(operation.get("edge_id"), "edge id")
                if edge_id not in edges:
                    raise ResearchMapError(f"unknown edge: {edge_id}")
                del edges[edge_id]
            elif op == "attach_evidence":
                node_id = _clean_id(operation.get("node_id"), "node id")
                if node_id not in nodes:
                    raise ResearchMapError(f"unknown node: {node_id}")
                refs = _bounded_strings(operation.get("source_refs"), "source_refs", 16)
                nodes[node_id]["source_refs"] = sorted(set(nodes[node_id].get("source_refs", [])) | set(refs))
                nodes[node_id]["updated_at"] = utc_now()
            elif op == "set_current_focus":
                node_id = operation.get("node_id")
                if node_id is not None:
                    node_id = _clean_id(node_id, "node id")
                    if node_id not in nodes:
                        raise ResearchMapError(f"unknown node: {node_id}")
                candidate["current_focus_node_id"] = node_id
        candidate["nodes"] = list(nodes.values())
        candidate["edges"] = list(edges.values())
        candidate["review_state"] = "reviewed" if actor == "user" else "needs_review"
        summary = _clean_text(patch.get("summary") or f"Applied {len(operations)} map operations", "patch summary", 500)
        result = write_research_map(
            config, workspace.workspace_id, candidate, actor=actor,
            expected_revision=base_revision, event_type="patch_applied", event_summary=summary,
        )
        return {
            "ok": True,
            "workspace_id": workspace.workspace_id,
            "revision": result["revision"],
            "operation_count": len(operations),
            "research_map": result,
        }


def save_layout(
    config: ServerConfig, workspace_id: str, layout: dict[str, Any], *, actor: str = "user",
) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    if not isinstance(layout, dict):
        raise ResearchMapError("layout must be an object")
    nodes = layout.get("nodes") or []
    viewport = layout.get("viewport") or {"x": 0, "y": 0, "zoom": 1}
    if not isinstance(nodes, list) or len(nodes) > 500 or not isinstance(viewport, dict):
        raise ResearchMapError("invalid layout")
    normalized_nodes = []
    known = {node["id"] for node in load_research_map(config, workspace.workspace_id)["nodes"]}
    for item in nodes:
        if not isinstance(item, dict):
            raise ResearchMapError("invalid layout node")
        node_id = _clean_id(item.get("id"), "layout node id")
        if node_id not in known:
            continue
        normalized_nodes.append({
            "id": node_id,
            "x": round(float(item.get("x", 0)), 2),
            "y": round(float(item.get("y", 0)), 2),
            "collapsed": bool(item.get("collapsed", False)),
        })
    value = {
        "schema_version": 1,
        "workspace_id": workspace.workspace_id,
        "nodes": normalized_nodes,
        "viewport": {
            "x": round(float(viewport.get("x", 0)), 2),
            "y": round(float(viewport.get("y", 0)), 2),
            "zoom": max(0.1, min(round(float(viewport.get("zoom", 1)), 3), 4.0)),
        },
        "updated_at": utc_now(),
        "updated_by": actor[:120],
    }
    _atomic_json(map_paths(workspace)["layout"], value)
    return {"ok": True, "workspace_id": workspace.workspace_id, "layout": value}


def load_layout(config: ServerConfig, workspace_id: str) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    path = map_paths(workspace)["layout"]
    if not path.is_file():
        return {
            "schema_version": 1, "workspace_id": workspace.workspace_id,
            "nodes": [], "viewport": {"x": 0, "y": 0, "zoom": 1},
            "updated_at": None, "updated_by": None,
        }
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ResearchMapError(f"research map layout could not be read: {error}") from error
    if not isinstance(value, dict) or value.get("workspace_id") != workspace.workspace_id:
        raise ResearchMapError("invalid research map layout")
    return value


def load_map_events(
    config: ServerConfig, workspace_id: str, *, limit: int = 50,
) -> list[dict[str, Any]]:
    """Read a bounded newest-first map audit timeline; malformed lines are ignored."""
    workspace = resolve_workspace(config, workspace_id)
    path = map_paths(workspace)["events"]
    if not path.is_file():
        return []
    bounded_limit = max(1, min(int(limit), 200))
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise ResearchMapError(f"research map events could not be read: {error}") from error
    events: list[dict[str, Any]] = []
    for line in reversed(lines[-500:]):
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(item, dict) and item.get("workspace_id") == workspace.workspace_id:
            events.append(item)
        if len(events) >= bounded_limit:
            break
    return events


def focus_capsule(value: dict[str, Any], *, max_nodes: int = 12) -> dict[str, Any]:
    nodes = _node_index(value)
    edges = value["edges"]
    focus_id = value.get("current_focus_node_id")
    roots = [node for node in value["nodes"] if node["type"] == "core_idea" and node["status"] != "archived"]
    active_claims = [node for node in value["nodes"] if node["type"] == "claim" and node["status"] not in {"archived", "rejected"}]
    blockers = [node for node in value["nodes"] if node["type"] == "risk" and node["status"] not in {"archived", "resolved"}]
    experiments = [node for node in value["nodes"] if node["type"] == "experiment" and node["status"] in {"planned", "running", "active"}]
    focus = nodes.get(focus_id) if focus_id else None
    related: list[str] = []
    if focus_id:
        for edge in edges:
            if edge["source"] == focus_id:
                related.append(edge["target"])
            elif edge["target"] == focus_id:
                related.append(edge["source"])
    return {
        "status": "ready" if value["nodes"] else "empty",
        "map_revision": value["revision"],
        "review_state": value["review_state"],
        "core_idea": ({"id": roots[0]["id"], "title": roots[0]["title"]} if roots else None),
        "active_claim": ({"id": active_claims[0]["id"], "title": active_claims[0]["title"], "status": active_claims[0]["status"]} if active_claims else None),
        "current_target": ({"id": focus["id"], "title": focus["title"], "status": focus["status"]} if focus else None),
        "next_experiment": ({"id": experiments[0]["id"], "title": experiments[0]["title"], "status": experiments[0]["status"]} if experiments else None),
        "blockers": [{"id": node["id"], "title": node["title"]} for node in blockers[:4]],
        "related_nodes": [
            {"id": nodes[node_id]["id"], "type": nodes[node_id]["type"], "title": nodes[node_id]["title"], "status": nodes[node_id]["status"]}
            for node_id in related[:max_nodes] if node_id in nodes
        ],
        "counts": {
            "nodes": len(value["nodes"]), "edges": len(edges),
            "active_branches": sum(1 for node in value["nodes"] if node["type"] == "branch" and node["status"] == "active"),
            "open_risks": len(blockers),
        },
        "updated_at": value["updated_at"],
    }


def research_map_bundle(config: ServerConfig, workspace_id: str) -> dict[str, Any]:
    value = load_research_map(config, workspace_id)
    return {
        "workspace_id": value["workspace_id"],
        "research_map": value,
        "layout": load_layout(config, workspace_id),
        "focus_capsule": focus_capsule(value),
        "events": load_map_events(config, workspace_id),
    }


def _first_text(value: Any, keys: tuple[str, ...] = ()) -> str:
    if isinstance(value, str):
        return " ".join(value.split())
    if isinstance(value, dict):
        for key in keys:
            text = _first_text(value.get(key))
            if text:
                return text
        for item in value.values():
            text = _first_text(item)
            if text:
                return text
    if isinstance(value, list):
        for item in value:
            text = _first_text(item)
            if text:
                return text
    return ""


def _safe_identifier(prefix: str, value: Any, fallback: str) -> str:
    body = re.sub(r"[^A-Za-z0-9_.:-]+", "-", str(value or fallback)).strip("-.")[:80]
    return f"{prefix}:{body or fallback}"


def _display_title(value: str, limit: int = 160) -> str:
    compact = " ".join(value.split())
    if len(compact) <= limit:
        return compact
    for separator in ("。", "？", "！", "; "):
        first = compact.split(separator, 1)[0].strip()
        if 24 <= len(first) <= limit:
            return first + (separator if separator in "。？！" else "")
    return compact[: limit - 1].rstrip() + "…"


def initialize_research_map(
    config: ServerConfig, workspace_id: str, *, actor: str = "system",
) -> dict[str, Any]:
    """Create a deterministic, review-required starter graph without invoking a model."""
    workspace = resolve_workspace(config, workspace_id)
    current = load_research_map(config, workspace.workspace_id)
    if current["nodes"]:
        return {
            "ok": True, "cached": True, "workspace_id": workspace.workspace_id,
            "revision": current["revision"], "research_map": current,
        }
    from .context import research_context_data
    from .index import query_experiments

    reviewed: dict[str, Any] = {}
    if workspace.context_file.is_file():
        try:
            import yaml
            loaded = yaml.safe_load(workspace.context_file.read_text(encoding="utf-8"))
            if isinstance(loaded, dict):
                reviewed = loaded
        except (OSError, yaml.YAMLError):
            pass
    research = research_context_data(
        config, workspace.workspace_id,
        sections=["objective", "hypotheses", "claims", "decisions", "open_questions"],
    )
    overview = _first_text(reviewed.get("overview"))
    objective = _first_text(
        research.get("objective"),
        ("objective", "goal", "one_sentence", "title", "summary"),
    ) or _first_text(reviewed.get("research_goal")) or overview or f"{workspace.name} 的核心研究问题"
    value = empty_research_map(workspace)
    nodes: list[dict[str, Any]] = []
    edges: list[dict[str, Any]] = []

    root = _normalize_node({
        "id": "idea:main", "type": "core_idea", "title": _display_title(objective),
        "summary": overview or objective, "status": "active",
        "authority": "canonical_fact" if research.get("coverage") == "structured_adapter" else "provisional",
        "origin": "structured_adapter" if research.get("coverage") == "structured_adapter" else "automatic",
        "source_refs": list(research.get("evidence_refs") or [])[:8],
    })
    nodes.append(root)

    claims = research.get("claims") or research.get("hypotheses") or []
    for index, claim in enumerate(claims[:8]):
        if not isinstance(claim, dict):
            continue
        title = _first_text(claim, ("text", "one_sentence", "claim", "title"))
        if not title:
            continue
        claim_id = _safe_identifier("claim", claim.get("claim_id"), str(index + 1))
        nodes.append(_normalize_node({
            "id": claim_id, "type": "claim", "title": title[:240],
            "summary": _first_text(claim.get("falsifier")),
            "status": str(claim.get("status") or "active")[:64],
            "authority": "canonical_fact", "origin": "structured_adapter",
            "source_refs": list(research.get("evidence_refs") or [])[:8],
        }))
        edges.append(_normalize_edge({
            "source": root["id"], "target": claim_id, "relation": "decomposes_into",
            "origin": "structured_adapter", "source_refs": list(research.get("evidence_refs") or [])[:8],
        }))

    questions = research.get("open_questions") if isinstance(research.get("open_questions"), dict) else {}
    reviewed_questions = [
        _first_text(item) for item in (reviewed.get("open_questions") or [])
        if _first_text(item)
    ] if isinstance(reviewed.get("open_questions"), list) else []
    pending = _first_text(questions.get("pending_decision"))
    next_evidence = _first_text(questions.get("next_evidence_tier"))
    current_stage = _first_text(reviewed.get("current_stage"))
    target_title = pending or next_evidence or (reviewed_questions[0] if reviewed_questions else "明确下一项可验证的研究目标")
    target = _normalize_node({
        "id": "target:current", "type": "current_target", "title": _display_title(target_title),
        "summary": (next_evidence if pending else current_stage), "status": "active",
        "authority": "working_hypothesis", "origin": "structured_adapter" if pending else "context_file",
        "source_refs": list(research.get("evidence_refs") or [])[:8],
    })
    nodes.append(target)
    edges.append(_normalize_edge({
        "source": root["id"], "target": target["id"], "relation": "decomposes_into",
        "origin": target["origin"], "source_refs": target["source_refs"],
    }))
    value["current_focus_node_id"] = target["id"]

    for index, question in enumerate(reviewed_questions[1:7], start=1):
        branch_id = f"branch:open-question-{index}"
        nodes.append(_normalize_node({
            "id": branch_id, "type": "branch", "title": _display_title(question),
            "summary": "来自工作区概述的待探索问题", "status": "active",
            "authority": "working_hypothesis", "origin": "context_file",
        }))
        edges.append(_normalize_edge({
            "source": target["id"], "target": branch_id, "relation": "decomposes_into",
            "origin": "context_file",
        }))

    for index, risk in enumerate((questions.get("unresolved_risks") or [])[:8]):
        text = _first_text(risk)
        if not text:
            continue
        risk_id = f"risk:{index + 1}"
        nodes.append(_normalize_node({
            "id": risk_id, "type": "risk", "title": text[:240], "status": "open",
            "authority": "canonical_fact", "origin": "structured_adapter",
            "source_refs": list(research.get("evidence_refs") or [])[:8],
        }))
        edges.append(_normalize_edge({
            "source": target["id"], "target": risk_id, "relation": "blocked_by",
            "origin": "structured_adapter", "source_refs": list(research.get("evidence_refs") or [])[:8],
        }))

    try:
        experiments = query_experiments(config, workspace, detail="summary", limit=5)["experiments"]
    except (OSError, ValueError, KeyError):
        experiments = []
    for index, experiment in enumerate(experiments[:5]):
        experiment_id = _safe_identifier("experiment", experiment.get("experiment_id"), str(index + 1))
        evidence = experiment.get("evidence") if isinstance(experiment.get("evidence"), dict) else {}
        nodes.append(_normalize_node({
            "id": experiment_id, "type": "experiment",
            "title": _first_text(experiment.get("title"))[:240] or str(experiment.get("experiment_id")),
            "summary": json.dumps(experiment.get("metrics") or {}, ensure_ascii=False)[:1200],
            "status": "completed", "authority": "observed_artifact", "origin": "experiment_index",
            "source_refs": [str(evidence.get("evidence_ref"))] if evidence.get("evidence_ref") else [],
            "metadata": {"experiment_id": experiment.get("experiment_id")},
        }))
        edges.append(_normalize_edge({
            "source": target["id"], "target": experiment_id, "relation": "motivated_by",
            "origin": "experiment_index",
        }))

    decision = research.get("decision")
    if isinstance(decision, dict) and decision:
        outcome = str(decision.get("outcome") or "UNDECIDED")
        decision_node = _normalize_node({
            "id": "decision:current", "type": "decision", "title": f"当前决策：{outcome}",
            "summary": _first_text(decision.get("rationale")), "status": outcome.casefold(),
            "authority": "canonical_fact", "origin": "structured_adapter",
            "source_refs": list(research.get("evidence_refs") or [])[:8],
        })
        nodes.append(decision_node)
        edges.append(_normalize_edge({
            "source": decision_node["id"], "target": root["id"], "relation": "derived_from",
            "origin": "structured_adapter", "source_refs": decision_node["source_refs"],
        }))

    value.update({"nodes": nodes, "edges": edges, "review_state": "needs_review"})
    written = write_research_map(
        config, workspace.workspace_id, value, actor=actor, expected_revision=0,
        event_type="initialized", event_summary="Initialized a deterministic research-map draft",
    )
    return {
        "ok": True, "cached": False, "workspace_id": workspace.workspace_id,
        "revision": written["revision"], "research_map": written,
    }


def codex_map_context(config: ServerConfig, workspace_id: str) -> dict[str, Any]:
    value = load_research_map(config, workspace_id)
    capsule = focus_capsule(value)
    active = [
        {
            "id": node["id"], "type": node["type"], "title": node["title"],
            "status": node["status"], "summary": node["summary"][:500],
            "authority": node["authority"], "source_refs": node["source_refs"][:8],
        }
        for node in value["nodes"]
        if node["status"] != "archived"
    ][:40]
    return {
        "workspace_id": value["workspace_id"],
        "map_revision": value["revision"],
        "review_state": value["review_state"],
        "focus": capsule,
        "active_nodes": active,
        "edges": [
            {key: edge[key] for key in ("id", "source", "target", "relation", "label")}
            for edge in value["edges"]
            if edge["source"] in {node["id"] for node in active}
            and edge["target"] in {node["id"] for node in active}
        ][:80],
        "update_protocol": {
            "format": "domain_map_patch_v1",
            "operations": sorted(PATCH_OPERATIONS),
            "rule": "Return a proposal only. Never edit layout or silently promote claims/decisions.",
        },
    }


def _ensure_proposal_table(config: ServerConfig) -> None:
    with _connect(config) as connection:
        connection.execute("""CREATE TABLE IF NOT EXISTS research_map_proposals (
            proposal_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, base_revision INTEGER NOT NULL,
            source_session_id TEXT, source_kind TEXT NOT NULL, status TEXT NOT NULL,
            patch_json TEXT, summary TEXT, error TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        )""")
        connection.execute(
            "CREATE INDEX IF NOT EXISTS research_map_proposals_workspace_updated "
            "ON research_map_proposals(workspace_id,updated_at DESC)"
        )


def _validate_patch_envelope(patch: Any, current_revision: int | None = None) -> dict[str, Any]:
    if not isinstance(patch, dict):
        raise ResearchMapError("patch must be an object")
    operations = patch.get("operations")
    if not isinstance(operations, list) or not operations or len(operations) > 100:
        raise ResearchMapError("patch operations must contain 1 to 100 items")
    for operation in operations:
        if not isinstance(operation, dict) or operation.get("op") not in PATCH_OPERATIONS:
            raise ResearchMapError("patch contains an unsupported operation")
    base_revision = int(patch.get("base_revision", -1))
    if base_revision < 0:
        raise ResearchMapError("patch base_revision is required")
    if current_revision is not None and base_revision != current_revision:
        raise ResearchMapConflict(
            f"revision_conflict: expected {base_revision}, current {current_revision}"
        )
    return {
        "base_revision": base_revision,
        "summary": _clean_text(patch.get("summary") or "Research map update", "patch summary", 500),
        "operations": operations,
    }


def create_map_proposal(
    config: ServerConfig, workspace_id: str, patch: dict[str, Any], *,
    source_kind: str, source_session_id: str | None = None,
) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    current = load_research_map(config, workspace.workspace_id)
    normalized = _validate_patch_envelope(patch, current["revision"])
    encoded = json.dumps(normalized, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    proposal_id = "mapprop_" + hashlib.sha256(
        f"{workspace.workspace_id}\0{source_kind}\0{source_session_id}\0{encoded}".encode()
    ).hexdigest()[:20]
    now = utc_now()
    _ensure_proposal_table(config)
    with _connect(config) as connection:
        connection.execute(
            """INSERT INTO research_map_proposals
               (proposal_id,workspace_id,base_revision,source_session_id,source_kind,status,
                patch_json,summary,error,created_at,updated_at)
               VALUES(?,?,?,?,?,'pending',?,?,NULL,?,?)
               ON CONFLICT(proposal_id) DO UPDATE SET updated_at=excluded.updated_at""",
            (proposal_id, workspace.workspace_id, normalized["base_revision"], source_session_id,
             source_kind[:80], encoded, normalized["summary"], now, now),
        )
    return {
        "proposal_id": proposal_id, "workspace_id": workspace.workspace_id,
        "base_revision": normalized["base_revision"], "source_session_id": source_session_id,
        "source_kind": source_kind, "status": "pending", "patch": normalized,
        "summary": normalized["summary"], "created_at": now, "updated_at": now,
    }


def _proposal_dict(row: sqlite3.Row) -> dict[str, Any]:
    patch = None
    if row["patch_json"]:
        try:
            patch = json.loads(row["patch_json"])
        except json.JSONDecodeError:
            patch = None
    return {
        "proposal_id": row["proposal_id"], "workspace_id": row["workspace_id"],
        "base_revision": row["base_revision"], "source_session_id": row["source_session_id"],
        "source_kind": row["source_kind"], "status": row["status"], "patch": patch,
        "summary": row["summary"], "error": row["error"],
        "created_at": row["created_at"], "updated_at": row["updated_at"],
    }


def list_map_proposals(
    config: ServerConfig, workspace_id: str, *, limit: int = 30,
) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    _ensure_proposal_table(config)
    with _connect(config) as connection:
        rows = connection.execute(
            "SELECT * FROM research_map_proposals WHERE workspace_id=? ORDER BY updated_at DESC LIMIT ?",
            (workspace.workspace_id, max(1, min(int(limit), 100))),
        ).fetchall()
    return {"workspace_id": workspace.workspace_id, "proposals": [_proposal_dict(row) for row in rows]}


def apply_map_proposal(config: ServerConfig, workspace_id: str, proposal_id: str) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    _ensure_proposal_table(config)
    with _connect(config) as connection:
        row = connection.execute(
            "SELECT * FROM research_map_proposals WHERE proposal_id=? AND workspace_id=?",
            (proposal_id, workspace.workspace_id),
        ).fetchone()
    if row is None:
        raise ResearchMapError("unknown research-map proposal")
    if row["status"] != "pending" or not row["patch_json"]:
        raise ResearchMapError(f"proposal is not pending: {row['status']}")
    patch = json.loads(row["patch_json"])
    result = apply_map_patch(
        config, workspace.workspace_id, patch, actor="user",
        proposal_source=f"proposal:{proposal_id}",
    )
    with _connect(config) as connection:
        connection.execute(
            "UPDATE research_map_proposals SET status='applied',updated_at=? WHERE proposal_id=?",
            (utc_now(), proposal_id),
        )
    return {**result, "proposal_id": proposal_id, "proposal_status": "applied"}


def reject_map_proposal(config: ServerConfig, workspace_id: str, proposal_id: str) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    _ensure_proposal_table(config)
    with _connect(config) as connection:
        cursor = connection.execute(
            "UPDATE research_map_proposals SET status='rejected',updated_at=? "
            "WHERE proposal_id=? AND workspace_id=? AND status IN ('pending','failed')",
            (utc_now(), proposal_id, workspace.workspace_id),
        )
    if cursor.rowcount == 0:
        raise ResearchMapError("proposal was not found or cannot be rejected")
    return {"ok": True, "workspace_id": workspace.workspace_id,
            "proposal_id": proposal_id, "status": "rejected"}


def _latest_map_session(config: ServerConfig, workspace: WorkspaceConfig) -> dict[str, Any] | None:
    from .summarizer import _matching_sessions
    candidates = _matching_sessions(
        config, workspace, max_age_days=config.summarizer_recent_session_max_age_days,
    )
    rank = {"exact_cwd": 0, "descendant_cwd": 1, "ancestor_cwd": 2}
    candidates.sort(key=lambda item: (rank[item[2]], -item[0]))
    if not candidates:
        return None
    _, meta, match = candidates[0]
    return {
        "session_id": meta["session_id"], "updated_at": meta["updated_at"],
        "started_at": meta.get("started_at"), "workspace_match": match,
    }


def _review_prompt(config: ServerConfig, workspace: WorkspaceConfig, session: dict[str, Any]) -> str:
    command = str(Path(config.summarizer_command[0]).resolve())
    return (
        "Review the LabContext research map for this exact workspace using the context of this Codex "
        "session and the current project evidence. Do not apply changes directly and do not edit layout. "
        f"First run `{command} map context {workspace.workspace_id}`. Propose only material changes "
        "to the core idea, claims, explored branches, current target, experiments, evidence, decisions, "
        "or risks. Keep unverified judgments as codex_inference and preserve negative branches. Build a "
        "domain_map_patch_v1 JSON object with base_revision, summary, and operations. Submit it for human "
        f"review by running `{command} map propose {workspace.workspace_id} <patch-file> "
        f"--session-id {session['session_id']}`. Do not run map apply."
    )


def _reserve_generating_proposal(
    config: ServerConfig, workspace: WorkspaceConfig, session_id: str | None,
) -> str:
    _ensure_proposal_table(config)
    current = load_research_map(config, workspace.workspace_id)
    now = utc_now()
    proposal_id = "mapprop_" + hashlib.sha256(
        f"{workspace.workspace_id}\0worker\0{session_id}\0{current['revision']}\0{now}".encode()
    ).hexdigest()[:20]
    with _connect(config) as connection:
        connection.execute(
            """INSERT INTO research_map_proposals
               (proposal_id,workspace_id,base_revision,source_session_id,source_kind,status,
                patch_json,summary,error,created_at,updated_at)
               VALUES(?,?,?,?,?,'generating',NULL,'Codex is reviewing the research map',NULL,?,?)""",
            (proposal_id, workspace.workspace_id, current["revision"], session_id,
             "analysis_worker_fallback", now, now),
        )
    return proposal_id


def _run_map_review_worker(
    config: ServerConfig, workspace_id: str, proposal_id: str, session_id: str | None,
) -> None:
    workspace = resolve_workspace(config, workspace_id)
    try:
        context = codex_map_context(config, workspace.workspace_id)
        from .summarizer import _latest_session
        latest = _latest_session(config, workspace)
        excerpt = str(latest.get("excerpt") or "")[:config.summarizer_max_session_chars]
        prompt = (
            "Review this research map against the registered workspace and recent visible Codex-session "
            "context. Return only a domain_map_patch_v1 object conforming to the output schema. Propose "
            "material, evidence-aware changes; do not modify files, run experiments, use the network, or "
            "promote inferences to verified facts. Preserve explored negative branches. If no semantic "
            "change is justified, add a concise current_target update rather than inventing evidence.\n\n"
            f"<research_map_context>\n{json.dumps(context, ensure_ascii=False, indent=2)}\n</research_map_context>\n"
            f"<latest_session id={json.dumps(session_id)}>\n{excerpt}\n</latest_session>"
        )
        with tempfile.TemporaryDirectory(prefix="labcontext-map-review-") as temp:
            temp_path = Path(temp)
            schema_path = temp_path / "schema.json"
            output_path = temp_path / "patch.json"
            schema_path.write_text(json.dumps(MAP_PATCH_SCHEMA), encoding="utf-8")
            command = [
                *config.summarizer_command, "exec", "--ephemeral", "--skip-git-repo-check",
                "--sandbox", "read-only", "--model", config.summarizer_model,
                "--config", f'model_reasoning_effort="{config.summarizer_reasoning_effort}"',
                "--output-schema", str(schema_path), "--output-last-message", str(output_path),
                "--color", "never", "-C", str(workspace.root), "-",
            ]
            result = subprocess.run(
                command, input=prompt, text=True, capture_output=True, check=False,
                timeout=config.summarizer_timeout_seconds,
            )
            if result.returncode != 0:
                raise RuntimeError(f"Codex review worker exited with code {result.returncode}")
            patch = _validate_patch_envelope(
                json.loads(output_path.read_text(encoding="utf-8")),
                load_research_map(config, workspace.workspace_id)["revision"],
            )
        now = utc_now()
        with _connect(config) as connection:
            connection.execute(
                """UPDATE research_map_proposals SET status='pending',patch_json=?,summary=?,
                   updated_at=?,error=NULL WHERE proposal_id=?""",
                (json.dumps(patch, ensure_ascii=False, sort_keys=True), patch["summary"], now, proposal_id),
            )
    except Exception as error:
        with _connect(config) as connection:
            connection.execute(
                "UPDATE research_map_proposals SET status='failed',error=?,updated_at=? WHERE proposal_id=?",
                (str(error)[:1000], utc_now(), proposal_id),
            )


def review_with_latest_session(
    config: ServerConfig, workspace_id: str, *, prefer_queue: bool = True,
) -> dict[str, Any]:
    workspace = resolve_workspace(config, workspace_id)
    session = _latest_map_session(config, workspace)
    if prefer_queue and session is not None:
        prompt = _review_prompt(config, workspace, session)
        command = [
            config.summarizer_command[0], "queue", "--thread", session["session_id"],
            "--message", prompt,
        ]
        try:
            result = subprocess.run(command, capture_output=True, text=True, check=False, timeout=15)
        except (OSError, subprocess.TimeoutExpired):
            result = None
        if result is not None and result.returncode == 0:
            return {
                "workspace_id": workspace.workspace_id, "status": "queued_to_session",
                "session": session,
                "message": "The latest matching Codex session will submit a proposal for human review.",
            }
    proposal_id = _reserve_generating_proposal(
        config, workspace, session["session_id"] if session else None,
    )
    threading.Thread(
        target=_run_map_review_worker,
        args=(config, workspace.workspace_id, proposal_id, session["session_id"] if session else None),
        daemon=True,
    ).start()
    return {
        "workspace_id": workspace.workspace_id, "status": "worker_started",
        "proposal_id": proposal_id, "session": session,
        "message": "The latest session could not receive a queued message; an isolated Codex worker is reviewing the map.",
    }
