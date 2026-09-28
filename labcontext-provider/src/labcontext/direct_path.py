from __future__ import annotations

from datetime import datetime, timezone
import fnmatch
from pathlib import Path
from typing import Any

from .config import ServerConfig
from .file_types import (
    DEFAULT_BLOCKED_PARTS, INSPECT_VIEWS, MAX_INSPECT_FILE_BYTES,
    is_sensitive_file as _is_sensitive_file, readable_file as _readable_file,
    reader_kind as _reader_kind,
)
from .file_reader import bound_response, inspect_supported_file

DIRECTORY_VIEWS = {"auto", "tree"}
FILE_VIEWS = INSPECT_VIEWS
DIRECT_BLOCKED_PARTS = DEFAULT_BLOCKED_PARTS


def _requested_path(value: str) -> Path:
    raw = str(value).strip()
    if not raw or len(raw) > 4000 or "\x00" in raw:
        raise ValueError("path must contain 1 to 4000 characters")
    path = Path(raw).expanduser()
    if not path.is_absolute():
        raise ValueError("inspect_path requires an absolute path")
    try:
        return path.resolve(strict=True)
    except (OSError, RuntimeError) as exc:
        raise ValueError(f"path does not exist or cannot be resolved: {path}") from exc


def _scope_root(config: ServerConfig, path: Path) -> Path:
    matches = [
        root for root in config.registry_allowed_roots
        if path == root or path.is_relative_to(root)
    ]
    if not matches:
        raise ValueError("path must remain below registry.allowed_roots")
    return max(matches, key=lambda item: len(item.parts))


def _is_denied(config: ServerConfig, scope_root: Path, path: Path) -> bool:
    try:
        relative = path.relative_to(scope_root)
    except ValueError:
        return True
    folded_parts = {part.casefold() for part in relative.parts}
    if folded_parts & {part.casefold() for part in DIRECT_BLOCKED_PARTS}:
        return True
    relative_text = relative.as_posix()
    return any(
        fnmatch.fnmatch(relative_text, pattern)
        or Path(relative_text).match(pattern)
        for pattern in config.project.denied_globs
    )


def _validate_path(config: ServerConfig, path: Path) -> Path:
    scope_root = _scope_root(config, path)
    if _is_denied(config, scope_root, path):
        raise ValueError("path is denied by direct-path policy")
    if path.is_file():
        if _is_sensitive_file(path):
            raise ValueError("sensitive credential files cannot be inspected")
    if not path.is_file() and not path.is_dir():
        raise ValueError("inspect_path supports regular files and directories only")
    return scope_root


def _directory_tree(
    config: ServerConfig,
    requested: Path,
    scope_root: Path,
    depth: int,
    max_entries: int,
) -> dict[str, Any]:
    entries: list[dict[str, Any]] = []
    skipped = 0
    truncated = False

    def visit(directory: Path, level: int) -> None:
        nonlocal skipped, truncated
        if truncated:
            return
        try:
            children = sorted(directory.iterdir(), key=lambda item: (not item.is_dir(), item.name.casefold()))
        except OSError:
            skipped += 1
            return
        for child in children:
            if len(entries) >= max_entries:
                truncated = True
                return
            try:
                if child.is_symlink():
                    entries.append({
                        "path": child.relative_to(requested).as_posix(),
                        "type": "symlink",
                    })
                    continue
                resolved = child.resolve(strict=True)
                if not resolved.is_relative_to(scope_root) or _is_denied(config, scope_root, resolved):
                    skipped += 1
                    continue
                stat = resolved.stat()
            except (OSError, RuntimeError, ValueError):
                skipped += 1
                continue
            relative = resolved.relative_to(requested).as_posix()
            if resolved.is_dir():
                entries.append({
                    "path": relative,
                    "type": "directory",
                    "modified_at": datetime.fromtimestamp(stat.st_mtime, timezone.utc).isoformat(),
                })
                if level < depth:
                    visit(resolved, level + 1)
            elif resolved.is_file():
                if _is_sensitive_file(resolved):
                    skipped += 1
                    continue
                entries.append({
                    "path": relative,
                    "type": "file",
                    "size_bytes": stat.st_size,
                    "modified_at": datetime.fromtimestamp(stat.st_mtime, timezone.utc).isoformat(),
                    "text_readable": _readable_file(resolved) and stat.st_size <= MAX_INSPECT_FILE_BYTES,
                    "reader": _reader_kind(resolved),
                })

    visit(requested, 1)
    return {
        "scope": "direct_path",
        "path": str(requested),
        "path_type": "directory",
        "view": "tree",
        "depth": depth,
        "entry_count": len(entries),
        "entries": entries,
        "truncated": truncated,
        "skipped_by_policy_or_error": skipped,
        "allowed_root": str(scope_root),
    }


def _file_view(
    config: ServerConfig,
    requested: Path,
    scope_root: Path,
    view: str,
    query: str | None,
    start_line: int | None,
    end_line: int | None,
    start_page: int | None,
    end_page: int | None,
    json_pointer: str | None,
    max_chars: int,
) -> dict[str, Any]:
    response_cap = max(500, config.project.max_response_bytes - 2500)
    result = inspect_supported_file(
        requested, view, query, start_line, end_line, start_page, end_page,
        json_pointer, min(max_chars, response_cap),
    )
    result["file"]["modified_at"] = datetime.fromtimestamp(requested.stat().st_mtime, timezone.utc).isoformat()
    return bound_response({
        "scope": "direct_path", "path": str(requested), "path_type": "file",
        "allowed_root": str(scope_root), "authority": "observed_artifact", **result,
    }, config.project.max_response_bytes)


def inspect_path_data(
    config: ServerConfig,
    path: str,
    view: str = "auto",
    query: str | None = None,
    start_line: int | None = None,
    end_line: int | None = None,
    start_page: int | None = None,
    end_page: int | None = None,
    json_pointer: str | None = None,
    depth: int = 2,
    max_entries: int = 200,
    max_chars: int = 8000,
) -> dict[str, Any]:
    """Inspect an absolute file or directory below an allowed root without registering it."""
    requested = _requested_path(path)
    scope_root = _validate_path(config, requested)
    normalized_view = str(view).strip().lower()
    if requested.is_dir():
        if normalized_view not in DIRECTORY_VIEWS:
            raise ValueError("directory inspection supports view=auto or view=tree")
        return _directory_tree(
            config,
            requested,
            scope_root,
            max(1, min(int(depth), 3)),
            max(1, min(int(max_entries), 400)),
        )
    if normalized_view not in FILE_VIEWS:
        raise ValueError(f"file view must be one of: {', '.join(sorted(FILE_VIEWS))}")
    return _file_view(
        config,
        requested,
        scope_root,
        normalized_view,
        query,
        start_line,
        end_line,
        start_page,
        end_page,
        json_pointer,
        max_chars,
    )
