from __future__ import annotations

from datetime import datetime, timezone
import fnmatch
import hashlib
import json
from pathlib import Path
import re
from typing import Any

from .config import ServerConfig
from .context import (
    DEFAULT_BLOCKED_PARTS,
    INSPECT_VIEWS,
    MAX_INSPECT_FILE_BYTES,
    SENSITIVE_FILE_NAMES,
    SENSITIVE_FILE_SUFFIXES,
    TEXT_SUFFIXES,
    _json_outline,
    _json_pointer_value,
)
from .document_extract import (
    DOCUMENT_SUFFIXES,
    HTML_SUFFIXES,
    PDF_SUFFIXES,
    inspect_html_document,
    inspect_pdf_document,
)


DIRECTORY_VIEWS = {"auto", "tree"}
FILE_VIEWS = INSPECT_VIEWS | {"auto", "pages"}
TEXT_FILE_NAMES = {
    "dockerfile", "gemfile", "license", "makefile", "procfile", "readme",
}
DIRECT_BLOCKED_PARTS = DEFAULT_BLOCKED_PARTS | {
    ".aws", ".azure", ".gnupg", ".kube", ".ssh",
}


def _is_sensitive_file(path: Path) -> bool:
    name = path.name.casefold()
    return (
        name.startswith(".env")
        or name in SENSITIVE_FILE_NAMES
        or path.suffix.casefold() in SENSITIVE_FILE_SUFFIXES
    )


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


def _text_file(path: Path) -> bool:
    name = path.name.casefold()
    return path.suffix.casefold() in TEXT_SUFFIXES or name in TEXT_FILE_NAMES


def _readable_file(path: Path) -> bool:
    return _text_file(path) or path.suffix.casefold() in DOCUMENT_SUFFIXES


def _reader_kind(path: Path) -> str | None:
    suffix = path.suffix.casefold()
    if suffix in PDF_SUFFIXES:
        return "pdf"
    if suffix in HTML_SUFFIXES:
        return "html"
    if _text_file(path):
        return "text"
    return None


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


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
    stat = requested.stat()
    if stat.st_size > MAX_INSPECT_FILE_BYTES:
        raise ValueError(f"file exceeds the {MAX_INSPECT_FILE_BYTES} byte inspection limit")
    response_cap = max(500, config.project.max_response_bytes - 2500)
    max_chars = max(500, min(int(max_chars), 10_000, response_cap))
    base = {
        "scope": "direct_path",
        "path": str(requested),
        "path_type": "file",
        "view": view,
        "allowed_root": str(scope_root),
        "file": {
            "suffix": requested.suffix.casefold(),
            "size_bytes": stat.st_size,
            "modified_at": datetime.fromtimestamp(stat.st_mtime, timezone.utc).isoformat(),
            "sha256": _file_sha256(requested),
            "reader": _reader_kind(requested),
        },
        "authority": "observed_artifact",
    }

    suffix = requested.suffix.casefold()
    if suffix in PDF_SUFFIXES:
        extracted = inspect_pdf_document(
            requested, view, query, start_page, end_page, max_chars,
        )
        return {**base, **extracted}
    if suffix in HTML_SUFFIXES:
        extracted = inspect_html_document(
            requested, view, query, start_line, end_line, max_chars,
        )
        return {**base, **extracted}
    if not _text_file(requested):
        raise ValueError("inspect_path supports configured text, HTML, and PDF formats only")

    raw = requested.read_bytes()
    text = raw.decode("utf-8", errors="replace")
    lines = text.splitlines()
    selected_view = "full_bounded" if view == "auto" else view
    if selected_view == "pages":
        raise ValueError("pages view is available only for PDF documents")
    base["view"] = selected_view
    base["file"]["line_count"] = len(lines)

    if selected_view == "search":
        needle = str(query or "").strip()
        if not needle or len(needle) > 300:
            raise ValueError("query must contain 1 to 300 characters for search view")
        matches: list[dict[str, Any]] = []
        remaining = max_chars
        for index, line in enumerate(lines, start=1):
            if needle.casefold() not in line.casefold():
                continue
            first, last = max(1, index - 2), min(len(lines), index + 2)
            excerpt = "\n".join(lines[first - 1:last])[:remaining]
            if not excerpt:
                break
            matches.append({
                "line": index,
                "location": {"start_line": first, "end_line": last},
                "content": excerpt,
            })
            remaining -= len(excerpt)
            if len(matches) >= 12 or remaining <= 0:
                break
        return {
            **base,
            "query": needle,
            "matches": matches,
            "match_count_returned": len(matches),
            "truncated": len(matches) >= 12 or remaining <= 0,
        }

    if selected_view == "lines":
        first = int(start_line or 1)
        last = int(end_line or min(len(lines), first + 79))
        if first < 1 or first > max(1, len(lines)) or last < first or last - first + 1 > 200:
            raise ValueError("lines view requires a valid range of at most 200 lines")
        last = min(last, len(lines))
        full_content = "\n".join(lines[first - 1:last])
        return {
            **base,
            "location": {"start_line": first, "end_line": last},
            "content": full_content[:max_chars],
            "truncated": len(full_content) > max_chars,
        }

    if selected_view == "json_pointer":
        if requested.suffix.casefold() != ".json":
            raise ValueError("json_pointer view requires a .json file")
        try:
            value = json.loads(text)
        except json.JSONDecodeError as exc:
            raise ValueError(f"file is not valid JSON: {exc}") from exc
        pointer = str(json_pointer if json_pointer is not None else "")
        selected = _json_pointer_value(value, pointer)
        content = json.dumps(selected, ensure_ascii=False, indent=2)
        return {
            **base,
            "json_pointer": pointer,
            "content": content[:max_chars],
            "selected_shape": _json_outline(selected),
            "truncated": len(content) > max_chars,
        }

    if selected_view == "outline":
        suffix = requested.suffix.casefold()
        if suffix == ".json":
            try:
                outline: Any = _json_outline(json.loads(text))
            except json.JSONDecodeError as exc:
                raise ValueError(f"file is not valid JSON: {exc}") from exc
        elif suffix == ".md":
            headings = [
                {"line": index, "level": len(match.group(1)), "title": match.group(2).strip()[:300]}
                for index, line in enumerate(lines, start=1)
                if (match := re.match(r"^(#{1,6})\s+(.+?)\s*$", line))
            ]
            outline = {
                "type": "markdown",
                "headings": headings[:60],
                "headings_truncated": len(headings) > 60,
            }
        else:
            symbols = [
                {"line": index, "text": line.strip()[:300]}
                for index, line in enumerate(lines, start=1)
                if re.match(r"^\s*(class|def|async\s+def|fn|struct|enum|interface|function)\s+", line)
            ]
            outline = {
                "type": "text",
                "symbols": symbols[:60],
                "symbols_truncated": len(symbols) > 60,
            }
        return {
            **base,
            "outline": outline,
            "truncated": len(json.dumps(outline, ensure_ascii=False)) > max_chars,
        }

    content = text[:max_chars]
    returned_lines = min(len(lines), content.count("\n") + (1 if content else 0))
    return {
        **base,
        "location": {"start_line": 1, "end_line": returned_lines},
        "content": content,
        "truncated": len(text) > max_chars,
    }


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
