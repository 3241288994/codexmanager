"""Bounded, offline text extraction for notebooks and modern Office documents."""
from __future__ import annotations

import json
from pathlib import Path
import posixpath
from typing import Any
from xml.etree import ElementTree as ET
from zipfile import BadZipFile, ZipFile

MAX_EXTRACTED_CHARS = 1_000_000
MAX_ZIP_ENTRIES = 2000
MAX_ZIP_MEMBER_BYTES = 8_000_000
MAX_ZIP_TOTAL_BYTES = 32_000_000
W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
S = "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}"
A = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
P = "{http://schemas.openxmlformats.org/presentationml/2006/main}"
R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"


class _Text:
    def __init__(self) -> None:
        self.parts: list[str] = []
        self.remaining = MAX_EXTRACTED_CHARS
        self.truncated = False

    def add(self, text: str) -> bool:
        if not text:
            return True
        value = text + "\n"
        self.parts.append(value[:self.remaining])
        if len(value) > self.remaining:
            self.truncated = True
        self.remaining = max(0, self.remaining - len(value))
        return self.remaining > 0

    def value(self) -> str:
        return "".join(self.parts).rstrip()


def _joined(value: Any) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        return "".join(item for item in value if isinstance(item, str))
    return ""


def extract_notebook(path: Path) -> tuple[str, dict[str, Any], bool]:
    try:
        data = json.loads(path.read_text(encoding="utf-8-sig"))
    except (ValueError, UnicodeError) as exc:
        raise ValueError("Notebook must contain valid UTF-8 JSON") from exc
    if not isinstance(data, dict) or not isinstance(data.get("cells"), list):
        raise ValueError("Notebook must contain a cells array")
    text = _Text()
    cells_read = 0
    for index, cell in enumerate(data["cells"], 1):
        if not isinstance(cell, dict):
            continue
        cells_read += 1
        if not text.add(f"[Cell {index}: {cell.get('cell_type', 'unknown')}]"):
            break
        if not text.add(_joined(cell.get("source"))):
            break
        outputs = cell.get("outputs", [])
        if isinstance(outputs, list):
            for output in outputs:
                if not isinstance(output, dict):
                    continue
                plain = output.get("data", {})
                value = _joined(output.get("text")) or (
                    _joined(plain.get("text/plain")) if isinstance(plain, dict) else ""
                )
                if value and not text.add("[Text output]\n" + value):
                    break
        if text.remaining <= 0:
            break
    return text.value(), {
        "type": "notebook", "cell_count": len(data["cells"]), "cells_read": cells_read,
        "executed": False, "binary_outputs_omitted": True,
    }, text.truncated or cells_read < len(data["cells"])


def _xml(archive: ZipFile, name: str) -> ET.Element:
    try:
        info = archive.getinfo(name)
        if info.file_size > MAX_ZIP_MEMBER_BYTES:
            raise ValueError("Office XML part exceeds the extraction limit")
        with archive.open(info) as handle:
            raw = handle.read(MAX_ZIP_MEMBER_BYTES + 1)
        if len(raw) > MAX_ZIP_MEMBER_BYTES:
            raise ValueError("Office XML part exceeds the extraction limit")
        # Also catches UTF-16/32 declarations; no DTDs or custom entities are needed.
        normalized = raw.replace(b"\x00", b"").lower()
        if b"<!doctype" in normalized or b"<!entity" in normalized:
            raise ValueError("Office XML declarations containing DTDs/entities are unsupported")
        return ET.fromstring(raw)
    except (KeyError, ET.ParseError, RuntimeError) as exc:
        raise ValueError(f"Office XML part is missing, encrypted or invalid: {name}") from exc


def _relationships(archive: ZipFile, name: str, base: str) -> dict[str, str]:
    result = {}
    for node in _xml(archive, name):
        if node.get("TargetMode", "").casefold() == "external":
            continue
        target = node.get("Target", "")
        if "\\" in target or ":" in target:
            continue
        resolved = posixpath.normpath(target.lstrip("/") if target.startswith("/")
                                      else posixpath.join(base, target))
        if not resolved.startswith(base + "/") or ".." in resolved.split("/"):
            continue
        result[node.get("Id", "")] = resolved
    return result


def _word(archive: ZipFile, text: _Text) -> dict[str, Any]:
    document = _xml(archive, "word/document.xml")
    count = 0
    for paragraph in document.iter(W + "p"):
        pieces = []
        for node in paragraph.iter():
            if node.tag == W + "t":
                pieces.append(node.text or "")
            elif node.tag == W + "tab":
                pieces.append("\t")
            elif node.tag in {W + "br", W + "cr"}:
                pieces.append("\n")
        count += 1
        if not text.add("".join(pieces)):
            break
    return {"type": "docx", "paragraphs_read": count}


def _excel(archive: ZipFile, text: _Text) -> dict[str, Any]:
    strings = []
    if "xl/sharedStrings.xml" in archive.namelist():
        strings = ["".join(t.text or "" for t in node.iter(S + "t"))
                   for node in _xml(archive, "xl/sharedStrings.xml")]
    workbook = _xml(archive, "xl/workbook.xml")
    links = _relationships(archive, "xl/_rels/workbook.xml.rels", "xl")
    sheets = workbook.findall(f"{S}sheets/{S}sheet")
    names = []
    for sheet in sheets:
        name = sheet.get("name", "Sheet")[:200]
        target = links.get(sheet.get(R + "id", ""))
        if not target:
            raise ValueError("Workbook sheet does not refer to a local XML part")
        names.append(name)
        if not text.add(f"[Sheet: {name}]"):
            break
        for row in _xml(archive, target).iter(S + "row"):
            values = []
            for cell in row.findall(S + "c"):
                value = cell.findtext(S + "v", "")
                kind = cell.get("t")
                if kind == "s":
                    try:
                        index = int(value)
                        if index < 0:
                            raise ValueError()
                        value = strings[index]
                    except (ValueError, IndexError) as exc:
                        raise ValueError("Invalid workbook shared-string reference") from exc
                elif kind == "inlineStr":
                    value = "".join(node.text or "" for node in cell.iter(S + "t"))
                formula = cell.findtext(S + "f")
                if formula is not None:
                    value = f"={formula} [cached: {value}]"
                values.append(f"{cell.get('r', '?')}: {value}")
            if not text.add(" | ".join(values)):
                break
        if text.remaining <= 0:
            break
    return {"type": "xlsx", "sheet_count": len(sheets), "sheets_read": names,
            "values": "stored values; formulas are not evaluated; dates may be serial numbers"}


def _slides(archive: ZipFile, text: _Text) -> dict[str, Any]:
    presentation = _xml(archive, "ppt/presentation.xml")
    links = _relationships(archive, "ppt/_rels/presentation.xml.rels", "ppt")
    slides = presentation.findall(f"{P}sldIdLst/{P}sldId")
    count = 0
    for index, slide in enumerate(slides, 1):
        target = links.get(slide.get(R + "id", ""))
        if not target:
            raise ValueError("Slide does not refer to a local XML part")
        count += 1
        if not text.add(f"[Slide {index}]"):
            break
        for paragraph in _xml(archive, target).iter(A + "p"):
            if not text.add("".join(node.text or "" for node in paragraph.iter(A + "t"))):
                break
        if text.remaining <= 0:
            break
    return {"type": "pptx", "slide_count": len(slides), "slides_read": count}


def extract_office(path: Path) -> tuple[str, dict[str, Any], bool]:
    text = _Text()
    try:
        with ZipFile(path) as archive:
            members = archive.infolist()
            if len(members) > MAX_ZIP_ENTRIES or sum(m.file_size for m in members) > MAX_ZIP_TOTAL_BYTES:
                raise ValueError("Office archive exceeds the extraction limit")
            if len({m.filename for m in members}) != len(members):
                raise ValueError("Office archive contains duplicate parts")
            if any(m.flag_bits & 1 for m in members):
                raise ValueError("Encrypted Office archives are unsupported")
            reader = {".docx": _word, ".xlsx": _excel, ".pptx": _slides}[path.suffix.casefold()]
            metadata = reader(archive, text)
    except (BadZipFile, OSError, NotImplementedError) as exc:
        raise ValueError("Office document is not a readable DOCX/XLSX/PPTX archive") from exc
    return text.value(), metadata, text.truncated
