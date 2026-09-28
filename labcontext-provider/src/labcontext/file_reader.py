"""Shared file inspection; callers own authorization, this module owns parsing."""
from __future__ import annotations

import codecs
import hashlib
import json
from pathlib import Path
import re
from typing import Any, Iterator

from .file_types import INSPECT_VIEWS, MAX_INSPECT_FILE_BYTES, reader_kind
from .document_extract import (
    inspect_html_document, inspect_pdf_document, _pdf_reader, _page_text,
    MAX_PDF_PAGES,
)
from .structured_documents import extract_notebook, extract_office


def bound_response(result: dict[str, Any], max_bytes: int) -> dict[str, Any]:
    """Keep extracted content within the UTF-8 budget, including structured outlines."""
    def size(value: Any) -> int:
        return len(json.dumps(value, ensure_ascii=False).encode("utf-8"))
    if size(result) <= max_bytes:
        return result
    result["truncated"] = True
    for key in ("outline", "selected_shape", "document"):
        value = result.get(key)
        if isinstance(value, dict) and size(value) > max_bytes // 4:
            preview = json.dumps(value, ensure_ascii=False).encode("utf-8")[:max_bytes // 6].decode("utf-8", errors="ignore")
            result[key] = {"type": value.get("type", "document"), "preview": preview, "truncated": True}
    def contents(value: Any):
        if isinstance(value, dict):
            for key, child in value.items():
                if key in {"content", "preview", "snippet"} and isinstance(child, str) and child:
                    yield value, key
                elif isinstance(child, (dict, list)):
                    yield from contents(child)
        elif isinstance(value, list):
            for child in value:
                yield from contents(child)
    while (excess := size(result) - max_bytes) > 0:
        candidates = list(contents(result))
        if not candidates:
            if result.get("matches"):
                result["matches"].pop()
                result["match_count_returned"] = len(result["matches"])
                continue
            raise ValueError("Response metadata exceeds the configured response budget")
        container, key = max(candidates, key=lambda pair: len(pair[0][pair[1]].encode("utf-8")))
        raw = container[key].encode("utf-8")
        container[key] = raw[:max(0, len(raw) - excess - 32)].decode("utf-8", errors="ignore")
    return result


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate_file(path: Path) -> str:
    kind = reader_kind(path)
    if kind is None:
        raise ValueError("Unsupported file format or sensitive credential file")
    if not path.is_file() or path.stat().st_size > MAX_INSPECT_FILE_BYTES:
        raise ValueError(f"File must be regular and at most {MAX_INSPECT_FILE_BYTES} bytes")
    return kind


def read_text(path: Path) -> str:
    raw = path.read_bytes()
    if len(raw) > MAX_INSPECT_FILE_BYTES:
        raise ValueError("File exceeds the inspection limit")
    if raw.startswith((codecs.BOM_UTF32_LE, codecs.BOM_UTF32_BE)):
        encoding = "utf-32"
    elif raw.startswith((codecs.BOM_UTF16_LE, codecs.BOM_UTF16_BE)):
        encoding = "utf-16"
    else:
        encoding = "utf-8-sig"
    try:
        text = raw.decode(encoding)
    except UnicodeError:
        try:
            text = raw.decode("gb18030")
        except UnicodeError as exc:
            raise ValueError("File is not supported UTF-8, BOM UTF-16/32 or GB18030 text") from exc
    text = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", text)
    if any(ord(c) < 32 and c not in "\t\n\r\f\b" for c in text):
        raise ValueError("Binary control bytes are not supported in text files")
    return text


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


def _inspect_text(
    requested: Path, text: str, view: str, query: str | None,
    start_line: int | None, end_line: int | None,
    json_pointer: str | None, max_chars: int,
) -> dict[str, Any]:
    base: dict[str, Any] = {"file": {}}
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


def inspect_supported_file(
    path: Path, view: str = "auto", query: str | None = None,
    start_line: int | None = None, end_line: int | None = None,
    start_page: int | None = None, end_page: int | None = None,
    json_pointer: str | None = None, max_chars: int = 8000,
) -> dict[str, Any]:
    kind = validate_file(path)
    view = str(view).strip().lower()
    if view not in INSPECT_VIEWS:
        raise ValueError(f"view must be one of: {', '.join(sorted(INSPECT_VIEWS))}")
    max_chars = max(500, min(int(max_chars), 10_000))
    metadata = {"reader": kind, "sha256": file_sha256(path),
                "size_bytes": path.stat().st_size, "suffix": path.suffix.casefold()}
    if kind == "pdf":
        result = inspect_pdf_document(path, view, query, start_page, end_page, max_chars)
    elif kind == "html":
        result = inspect_html_document(path, view, query, start_line, end_line, max_chars)
    else:
        document = None
        extraction_truncated = False
        if kind == "notebook":
            text, document, extraction_truncated = extract_notebook(path)
        elif kind in {"docx", "xlsx", "pptx"}:
            text, document, extraction_truncated = extract_office(path)
        else:
            text = read_text(path)
        result = _inspect_text(path, text, view, query, start_line, end_line, json_pointer, max_chars)
        if document:
            result["document"] = document
            if view == "outline":
                result["outline"] = {
                    **document, "line_count": len(text.splitlines()),
                    "preview": text[:max_chars // 2],
                }
            result["extraction_truncated"] = extraction_truncated
            result["truncated"] = result.get("truncated", False) or extraction_truncated
    result["file"] = {**metadata, **result.get("file", {})}
    return result


def search_segments(path: Path) -> Iterator[tuple[str, dict[str, int], bool]]:
    """Yield canonical text and source coordinates; never index raw binary containers."""
    kind = validate_file(path)
    remaining = 200_000
    if kind == "pdf":
        reader = _pdf_reader(path)
        count = len(reader.pages)
        if not 1 <= count <= MAX_PDF_PAGES:
            raise ValueError("PDF page count exceeds the inspection limit")
        for page in range(1, count + 1):
            text = _page_text(reader, page)
            limited = text[:remaining]
            remaining -= len(limited)
            yield limited, {"start_page": page, "end_page": page}, len(text) > len(limited) or (remaining <= 0 and page < count)
            if remaining <= 0:
                break
        return
    if kind == "html":
        result = inspect_html_document(path, "full_bounded", None, None, None, remaining)
        text, truncated = result["content"], result["truncated"]
    elif kind == "notebook":
        text, _, truncated = extract_notebook(path)
    elif kind in {"docx", "xlsx", "pptx"}:
        text, _, truncated = extract_office(path)
    else:
        text, truncated = read_text(path), False
    yield text[:remaining], {}, truncated or len(text) > remaining


def read_evidence_content(path: Path, location: dict[str, Any], max_chars: int) -> str:
    kind = validate_file(path)
    if kind == "pdf" and location.get("start_page"):
        reader = _pdf_reader(path)
        first = int(location["start_page"])
        last = int(location.get("end_page") or first)
        if not 1 <= first <= last <= len(reader.pages) or last - first >= 20:
            raise ValueError("Invalid PDF evidence page range")
        parts = []
        for page in range(first, last + 1):
            text = _page_text(reader, page)
            if location.get("start_line"):
                lines = text.splitlines()
                text = "\n".join(lines[location["start_line"] - 1:location.get("end_line")])
            parts.append(f"[Page {page}]\n{text}")
        return "\n\n".join(parts)[:max_chars]
    start = location.get("start_line")
    end = location.get("end_line")
    result = inspect_supported_file(
        path, "lines" if start and kind != "pdf" else "full_bounded",
        start_line=start, end_line=min(end or start + 40, start + 199) if start else None,
        max_chars=max_chars,
    )
    return result.get("content", "")[:max_chars]
