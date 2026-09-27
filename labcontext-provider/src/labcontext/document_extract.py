from __future__ import annotations

from html.parser import HTMLParser
from pathlib import Path
import re
from typing import Any


HTML_SUFFIXES = {".htm", ".html"}
PDF_SUFFIXES = {".pdf"}
DOCUMENT_SUFFIXES = HTML_SUFFIXES | PDF_SUFFIXES
MAX_PDF_PAGES = 500
MAX_PDF_PAGES_PER_READ = 20
MAX_EXTRACTED_PAGE_CHARS = 200_000
MAX_SEARCH_MATCHES = 12


def _bounded(value: str, limit: int) -> tuple[str, bool]:
    if len(value) <= limit:
        return value, False
    return value[:limit].rstrip(), True


def _normalize_visible_text(value: str) -> str:
    value = value.replace("\r\n", "\n").replace("\r", "\n")
    value = re.sub(r"[\t\f\v ]+", " ", value)
    value = re.sub(r" *\n *", "\n", value)
    value = re.sub(r"\n{3,}", "\n\n", value)
    return value.strip()


class _VisibleHTMLParser(HTMLParser):
    _HIDDEN_TAGS = {"script", "style", "noscript", "svg", "template"}
    _BLOCK_TAGS = {
        "address", "article", "aside", "blockquote", "br", "caption", "dd", "div", "dl",
        "dt", "figcaption", "figure", "footer", "form", "header", "hr", "li", "main", "nav",
        "ol", "p", "pre", "section", "table", "tbody", "td", "tfoot", "th", "thead", "tr", "ul",
    }

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self._hidden_depth = 0
        self._pieces: list[str] = []
        self._capture_tag: str | None = None
        self._capture_parts: list[str] = []
        self.title = ""
        self.headings: list[dict[str, Any]] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        tag = tag.casefold()
        if tag in self._HIDDEN_TAGS:
            self._hidden_depth += 1
            return
        if self._hidden_depth:
            return
        if tag in self._BLOCK_TAGS or tag in {"h1", "h2", "h3", "h4", "h5", "h6"}:
            self._pieces.append("\n")
        if tag == "title" or re.fullmatch(r"h[1-6]", tag):
            self._capture_tag = tag
            self._capture_parts = []

    def handle_endtag(self, tag: str) -> None:
        tag = tag.casefold()
        if tag in self._HIDDEN_TAGS:
            if self._hidden_depth:
                self._hidden_depth -= 1
            return
        if self._hidden_depth:
            return
        if self._capture_tag == tag:
            text = re.sub(r"\s+", " ", "".join(self._capture_parts)).strip()
            if tag == "title":
                self.title = text[:500]
            elif text:
                self.headings.append({"level": int(tag[1]), "title": text[:500]})
            self._capture_tag = None
            self._capture_parts = []
        if tag in self._BLOCK_TAGS or re.fullmatch(r"h[1-6]", tag):
            self._pieces.append("\n")

    def handle_data(self, data: str) -> None:
        if self._hidden_depth or not data:
            return
        self._pieces.append(data)
        if self._capture_tag is not None:
            self._capture_parts.append(data)

    def visible_text(self) -> str:
        return _normalize_visible_text("".join(self._pieces))


def _search_lines(lines: list[str], query: str, max_chars: int) -> dict[str, Any]:
    needle = str(query or "").strip()
    if not needle or len(needle) > 300:
        raise ValueError("query must contain 1 to 300 characters for search view")
    matches: list[dict[str, Any]] = []
    remaining = max_chars
    for index, line in enumerate(lines, start=1):
        if needle.casefold() not in line.casefold():
            continue
        first, last = max(1, index - 2), min(len(lines), index + 2)
        excerpt, clipped = _bounded("\n".join(lines[first - 1:last]), remaining)
        if not excerpt:
            break
        matches.append({
            "line": index,
            "location": {"start_line": first, "end_line": last},
            "content": excerpt,
        })
        remaining -= len(excerpt)
        if len(matches) >= MAX_SEARCH_MATCHES or remaining <= 0 or clipped:
            break
    return {
        "query": needle,
        "matches": matches,
        "match_count_returned": len(matches),
        "truncated": len(matches) >= MAX_SEARCH_MATCHES or remaining <= 0,
    }


def inspect_html_document(
    path: Path,
    view: str,
    query: str | None,
    start_line: int | None,
    end_line: int | None,
    max_chars: int,
) -> dict[str, Any]:
    try:
        raw = path.read_text(encoding="utf-8", errors="replace")
        parser = _VisibleHTMLParser()
        parser.feed(raw)
        parser.close()
    except (OSError, ValueError) as exc:
        raise ValueError(f"HTML document could not be parsed: {exc}") from exc

    text = parser.visible_text()
    lines = text.splitlines()
    document = {
        "type": "html",
        "title": parser.title or None,
        "heading_count": len(parser.headings),
        "extracted_line_count": len(lines),
    }
    selected_view = "full_bounded" if view == "auto" else view

    if selected_view == "outline":
        headings = parser.headings[:100]
        return {
            "view": selected_view,
            "document": document,
            "outline": {
                "type": "html",
                "title": parser.title or None,
                "headings": headings,
                "headings_truncated": len(parser.headings) > len(headings),
            },
            "truncated": len(parser.headings) > len(headings),
        }
    if selected_view == "search":
        return {"view": selected_view, "document": document, **_search_lines(lines, query or "", max_chars)}
    if selected_view == "lines":
        first = int(start_line or 1)
        last = int(end_line or min(len(lines), first + 79))
        if first < 1 or first > max(1, len(lines)) or last < first or last - first + 1 > 200:
            raise ValueError("lines view requires a valid range of at most 200 extracted lines")
        last = min(last, len(lines))
        selected = "\n".join(lines[first - 1:last])
        content, clipped = _bounded(selected, max_chars)
        return {
            "view": selected_view,
            "document": document,
            "location": {"start_line": first, "end_line": last},
            "content": content,
            "truncated": clipped,
        }
    if selected_view not in {"full_bounded"}:
        raise ValueError("HTML documents support auto, outline, search, lines, or full_bounded views")
    content, clipped = _bounded(text, max_chars)
    returned_lines = min(len(lines), content.count("\n") + (1 if content else 0))
    return {
        "view": selected_view,
        "document": document,
        "location": {"start_line": 1, "end_line": returned_lines},
        "content": content,
        "truncated": clipped,
    }


def _pdf_reader(path: Path) -> Any:
    try:
        from pypdf import PdfReader
        from pypdf.errors import PdfReadError
    except ImportError as exc:
        raise ValueError("PDF support is unavailable because the pypdf dependency is not installed") from exc
    try:
        reader = PdfReader(str(path), strict=False)
        if reader.is_encrypted and reader.decrypt("") == 0:
            raise ValueError("encrypted PDF documents require a password and cannot be inspected")
        return reader
    except ValueError:
        raise
    except (OSError, PdfReadError) as exc:
        raise ValueError(f"PDF document could not be parsed: {exc}") from exc


def _pdf_metadata(reader: Any, page_count: int) -> dict[str, Any]:
    metadata = reader.metadata or {}

    def field(name: str) -> str | None:
        value = metadata.get(name)
        if value is None:
            return None
        return re.sub(r"\s+", " ", str(value)).strip()[:500] or None

    return {
        "type": "pdf",
        "page_count": page_count,
        "title": field("/Title"),
        "author": field("/Author"),
        "subject": field("/Subject"),
        "encrypted": bool(reader.is_encrypted),
    }


def _page_text(reader: Any, page_number: int) -> str:
    try:
        text = reader.pages[page_number - 1].extract_text() or ""
        return _normalize_visible_text(text[:MAX_EXTRACTED_PAGE_CHARS])
    except Exception as exc:
        raise ValueError(f"PDF page {page_number} could not be extracted: {exc}") from exc


def _pdf_page_range(
    page_count: int, start_page: int | None, end_page: int | None,
) -> tuple[int, int]:
    first = int(start_page or 1)
    last = int(end_page or min(page_count, first + 4))
    if first < 1 or first > max(1, page_count) or last < first:
        raise ValueError("pages view requires a valid 1-based PDF page range")
    if last - first + 1 > MAX_PDF_PAGES_PER_READ:
        raise ValueError(f"pages view reads at most {MAX_PDF_PAGES_PER_READ} PDF pages per call")
    return first, min(last, page_count)


def inspect_pdf_document(
    path: Path,
    view: str,
    query: str | None,
    start_page: int | None,
    end_page: int | None,
    max_chars: int,
) -> dict[str, Any]:
    reader = _pdf_reader(path)
    page_count = len(reader.pages)
    if page_count < 1:
        raise ValueError("PDF document has no pages")
    if page_count > MAX_PDF_PAGES:
        raise ValueError(f"PDF document exceeds the {MAX_PDF_PAGES}-page inspection limit")
    document = _pdf_metadata(reader, page_count)
    selected_view = "pages" if view in {"auto", "full_bounded"} else view

    if selected_view == "outline":
        return {
            "view": selected_view,
            "document": document,
            "outline": document,
            "truncated": False,
        }
    if selected_view == "search":
        needle = str(query or "").strip()
        if not needle or len(needle) > 300:
            raise ValueError("query must contain 1 to 300 characters for search view")
        first = int(start_page or 1)
        last = int(end_page or page_count)
        if first < 1 or first > page_count or last < first:
            raise ValueError("PDF search requires a valid 1-based page range")
        last = min(last, page_count)
        matches: list[dict[str, Any]] = []
        remaining = max_chars
        scanned = first - 1
        for page_number in range(first, last + 1):
            text = _page_text(reader, page_number)
            scanned = page_number
            folded = text.casefold()
            offset = folded.find(needle.casefold())
            if offset < 0:
                continue
            excerpt_start = max(0, offset - 240)
            excerpt_end = min(len(text), offset + len(needle) + 520)
            excerpt, clipped = _bounded(text[excerpt_start:excerpt_end].strip(), remaining)
            if not excerpt:
                break
            matches.append({"page": page_number, "content": excerpt})
            remaining -= len(excerpt)
            if len(matches) >= MAX_SEARCH_MATCHES or remaining <= 0 or clipped:
                break
        return {
            "view": selected_view,
            "document": document,
            "query": needle,
            "searched_pages": {"start_page": first, "end_page": scanned},
            "matches": matches,
            "match_count_returned": len(matches),
            "truncated": scanned < last or len(matches) >= MAX_SEARCH_MATCHES or remaining <= 0,
        }
    if selected_view != "pages":
        raise ValueError("PDF documents support auto, outline, search, pages, or full_bounded views")

    first, last = _pdf_page_range(page_count, start_page, end_page)
    chunks: list[str] = []
    remaining = max_chars
    extracted_pages = 0
    pages_with_text = 0
    clipped = False
    for page_number in range(first, last + 1):
        text = _page_text(reader, page_number)
        extracted_pages += 1
        if text:
            pages_with_text += 1
        chunk = f"[Page {page_number}]\n{text}".rstrip()
        bounded, page_clipped = _bounded(chunk, remaining)
        if bounded:
            chunks.append(bounded)
            remaining -= len(bounded) + 2
        if page_clipped or remaining <= 0:
            clipped = True
            break
    actual_last = first + extracted_pages - 1
    ocr_required = pages_with_text == 0
    return {
        "view": selected_view,
        "document": document,
        "location": {"start_page": first, "end_page": actual_last},
        "content": "\n\n".join(chunks),
        "text_status": "no_extractable_text" if ocr_required else "extracted",
        "ocr_required": ocr_required,
        "next_start_page": actual_last + 1 if actual_last < page_count else None,
        "truncated": clipped or actual_last < page_count,
    }
