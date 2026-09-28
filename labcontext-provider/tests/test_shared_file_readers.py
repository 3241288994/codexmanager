import asyncio
import json
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

import pytest

from labcontext.context import get_evidence_data, inspect_file_data, search_evidence_data
from labcontext.direct_path import inspect_path_data
from labcontext.service import build_server
from test_context_tools import configured_workspace, write_text_pdf


def zip_parts(path: Path, parts: dict[str, str]) -> None:
    with ZipFile(path, "w", compression=ZIP_DEFLATED) as archive:
        for name, content in parts.items():
            archive.writestr(name, content)


def make_document(path: Path) -> None:
    if path.suffix == ".docx":
        zip_parts(path, {"word/document.xml": '''<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Document verification</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>needlealpha</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>'''})
    elif path.suffix == ".xlsx":
        zip_parts(path, {
            "xl/workbook.xml": '''<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Results" sheetId="1" r:id="rId1"/></sheets></workbook>''',
            "xl/_rels/workbook.xml.rels": '''<Relationships><Relationship Id="rId1" Target="worksheets/sheet1.xml"/></Relationships>''',
            "xl/sharedStrings.xml": '''<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>needlealpha</t></si></sst>''',
            "xl/worksheets/sheet1.xml": '''<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1"><f>1+1</f><v>2</v></c></row></sheetData></worksheet>''',
        })
    elif path.suffix == ".pptx":
        zip_parts(path, {
            "ppt/presentation.xml": '''<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst><p:sldId id="256" r:id="slideB"/><p:sldId id="257" r:id="slideA"/></p:sldIdLst></p:presentation>''',
            "ppt/_rels/presentation.xml.rels": '''<Relationships><Relationship Id="slideA" Target="slides/slide1.xml"/><Relationship Id="slideB" Target="slides/slide2.xml"/></Relationships>''',
            "ppt/slides/slide1.xml": '''<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:p><a:r><a:t>SECOND slide</a:t></a:r></a:p></p:sld>''',
            "ppt/slides/slide2.xml": '''<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:p><a:r><a:t>needlealpha FIRST slide</a:t></a:r></a:p></p:sld>''',
        })
    elif path.suffix == ".ipynb":
        path.write_text(json.dumps({"nbformat": 4, "cells": [
            {"cell_type": "markdown", "source": ["# needlealpha"]},
            {"cell_type": "code", "source": "print(2)", "outputs": [
                {"output_type": "stream", "text": ["2\n"]},
                {"data": {"image/png": "HIDDEN_IMAGE_PAYLOAD", "text/plain": ["plot"]}},
            ]},
        ]}))
    elif path.suffix == ".html":
        path.write_text("<h1>Document</h1><p>needlealpha</p><script>HIDDEN_SCRIPT_PAYLOAD</script>")
    elif path.suffix == ".pdf":
        write_text_pdf(path, ["First page", "needlealpha on second page", "Last page"])


@pytest.mark.parametrize("name", [
    "app.js", "app.jsx", "app.mjs", "app.cjs", "app.tsx", "style.css", "style.scss",
    "app.vue", "app.svelte", "app.h", "app.hpp", "app.cs", "app.swift", "app.kt",
    "app.rb", "app.php", "app.lua", "app.sql", "app.proto", "settings.ini",
    "settings.conf", "settings.xml", "settings.properties", "app.log", "data.tsv",
    "notes.rst", "changes.diff", "diagram.svg", "run.ps1", "run.bat", "README",
    "LICENSE", "Dockerfile", "Dockerfile.dev", "Makefile", "Cargo.lock", "go.mod",
    ".gitignore", ".editorconfig",
])
def test_common_formats_read_search_and_evidence_roundtrip(tmp_path: Path, name: str):
    config = configured_workspace(tmp_path)
    path = config.workspaces["main"].root / "docs" / name
    path.write_text("First line\nneedlealpha shared reader\nLast line\n")
    direct = inspect_path_data(config, str(path), view="lines", start_line=2, end_line=2)
    workspace = inspect_file_data(config, str(path), view="lines", start_line=2, end_line=2)
    assert direct["content"] == workspace["content"] == "needlealpha shared reader"
    matches = search_evidence_data(config, "needlealpha", "main")["results"]
    match = next(m for m in matches if m["path"] == f"docs/{name}")
    evidence = get_evidence_data(config, [match["evidence_ref"]])["results"][0]
    assert evidence["status"] == "ok"
    assert "needlealpha shared reader" in evidence["content"]


@pytest.mark.parametrize("suffix", [".html", ".pdf", ".docx", ".xlsx", ".pptx", ".ipynb"])
def test_document_parity_search_and_evidence(tmp_path: Path, suffix: str):
    config = configured_workspace(tmp_path)
    path = config.workspaces["main"].root / "docs" / ("example" + suffix)
    make_document(path)
    direct = inspect_path_data(config, str(path), view="auto")
    workspace = inspect_file_data(config, str(path), view="auto")
    assert direct["content"] == workspace["content"]
    assert "needlealpha" in direct["content"]
    assert "HIDDEN_" not in direct["content"]
    if suffix == ".xlsx":
        assert "[Sheet: Results]" in direct["content"]
        assert "A1: needlealpha" in direct["content"]
        assert "B1: =1+1 [cached: 2]" in direct["content"]
    if suffix == ".pptx":
        assert direct["content"].index("FIRST") < direct["content"].index("SECOND")
    found = search_evidence_data(config, "needlealpha", "main")["results"]
    match = next(m for m in found if m["path"].endswith(suffix))
    evidence = get_evidence_data(config, [match["evidence_ref"]])["results"][0]
    assert evidence["status"] == "ok"
    assert "needlealpha" in evidence["content"]
    if suffix == ".pdf":
        assert evidence["location"]["start_page"] == 2
        assert "First page" not in evidence["content"]
    # The source file hash, not extracted text, determines evidence freshness.
    path.write_bytes(path.read_bytes() + b"\n")
    stale = get_evidence_data(config, [match["evidence_ref"]])["results"][0]
    assert stale["status"] == "stale_reference"
    assert stale["content"] == ""


def test_workspace_pdf_pages_and_schema(tmp_path: Path):
    config = configured_workspace(tmp_path)
    path = config.workspaces["main"].root / "docs" / "paper.pdf"
    make_document(path)
    result = inspect_file_data(config, str(path), view="pages", start_page=2, end_page=2)
    assert "needlealpha" in result["content"] and "First page" not in result["content"]
    evidence = get_evidence_data(config, [result["evidence_ref"]])["results"][0]
    assert "needlealpha" in evidence["content"] and "First page" not in evidence["content"]
    async def schema():
        tools = await build_server(config).get_tools()
        assert {"start_page", "end_page"} <= tools["inspect_file"].parameters["properties"].keys()
    asyncio.run(schema())


@pytest.mark.parametrize("encoding", ["utf-8-sig", "utf-16", "utf-32", "gb18030"])
def test_text_encodings_and_ansi_logs(tmp_path: Path, encoding: str):
    config = configured_workspace(tmp_path)
    path = config.workspaces["main"].root / "docs" / "service.log"
    path.write_bytes("\x1b[32m中文日志正常\x1b[0m\n".encode(encoding))
    a = inspect_path_data(config, str(path))
    b = inspect_file_data(config, str(path), view="auto")
    assert a["content"] == b["content"] == "中文日志正常\n"


@pytest.mark.parametrize("name", ["auth.json", "credentials.json", ".env.local", ".npmrc", "admin.token"])
def test_expanded_formats_do_not_expose_credentials(tmp_path: Path, name: str):
    config = configured_workspace(tmp_path)
    path = config.workspaces["main"].root / "docs" / name
    path.write_text("needlealpha secret")
    for read in (inspect_path_data, inspect_file_data):
        with pytest.raises(ValueError):
            read(config, str(path))
    assert not search_evidence_data(config, "needlealpha", "main")["results"]


@pytest.mark.parametrize("suffix", [".doc", ".xls", ".ppt", ".png", ".zip", ".docm"])
def test_unsupported_binary_formats_remain_blocked(tmp_path: Path, suffix: str):
    config = configured_workspace(tmp_path)
    path = config.workspaces["main"].root / "docs" / ("file" + suffix)
    path.write_bytes(b"unsupported")
    for read in (inspect_path_data, inspect_file_data):
        with pytest.raises(ValueError):
            read(config, str(path))


def test_binary_renamed_as_text_and_broken_documents(tmp_path: Path):
    config = configured_workspace(tmp_path)
    for name, payload in [("bad.js", b"abc\x00binary"), ("bad.docx", b"not a zip"),
                          ("bad.ipynb", b'{"cells": 42}')]:
        path = config.workspaces["main"].root / "docs" / name
        path.write_bytes(payload)
        for read in (inspect_path_data, inspect_file_data):
            with pytest.raises(ValueError):
                read(config, str(path))
    coverage = search_evidence_data(config, "needlealpha", "main")["coverage"]
    assert coverage["files_skipped_unreadable"] == 3


def test_docx_dtd_and_zip_expansion_limits(tmp_path: Path):
    config = configured_workspace(tmp_path)
    path = config.workspaces["main"].root / "docs" / "bad.docx"
    for payload in [
        '<!DOCTYPE d [<!ENTITY x "expanded">]><d>&x;</d>',
        "X" * 8_000_001,
    ]:
        zip_parts(path, {"word/document.xml": payload})
        for read in (inspect_path_data, inspect_file_data):
            with pytest.raises(ValueError):
                read(config, str(path))


def test_pdf_scanned_requires_ocr_in_both_routes(tmp_path: Path):
    config = configured_workspace(tmp_path)
    path = config.workspaces["main"].root / "docs" / "scan.pdf"
    write_text_pdf(path, [""])
    for read in (inspect_path_data, inspect_file_data):
        assert read(config, str(path), view="auto")["ocr_required"] is True


@pytest.mark.parametrize("view", ["full_bounded", "outline", "search"])
def test_unicode_and_long_outline_respect_response_bytes(tmp_path: Path, view: str):
    config = configured_workspace(tmp_path)
    path = config.workspaces["main"].root / "docs" / "long.md"
    path.write_text("\n".join("# 中文标题" + "内容" * 200 for _ in range(90)))
    for read in (inspect_path_data, inspect_file_data):
        result = read(config, str(path), view=view, query="中文", max_chars=10000)
        assert len(json.dumps(result, ensure_ascii=False).encode()) <= config.project.max_response_bytes
        assert result["truncated"] is True


def test_notebook_output_truncation_and_directory_readers(tmp_path: Path):
    config = configured_workspace(tmp_path)
    docs = config.workspaces["main"].root / "docs"
    path = docs / "large.ipynb"
    path.write_text(json.dumps({"cells": [{"cell_type": "code", "source": "x" * 1_100_000}]}))
    result = inspect_file_data(config, str(path), view="auto")
    assert result["extraction_truncated"] is True and result["truncated"] is True
    for suffix in (".docx", ".xlsx", ".pptx"):
        make_document(docs / ("sample" + suffix))
    tree = inspect_path_data(config, str(docs), view="tree")
    entries = {entry["path"]: entry for entry in tree["entries"]}
    assert entries["sample.xlsx"]["reader"] == "xlsx"
    assert entries["large.ipynb"]["reader"] == "notebook"
