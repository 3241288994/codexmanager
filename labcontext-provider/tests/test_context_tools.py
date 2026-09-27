import asyncio
import json
import os
from pathlib import Path
import time

from labcontext.config import load_config
from labcontext.context import (
    get_evidence_data, inspect_file_data, list_workspaces_data, research_context_data,
    search_evidence_data, workspace_overview_data,
)
from labcontext.direct_path import inspect_path_data
from labcontext.index import compare_experiments, query_experiments
from labcontext.context import resolve_workspace
from labcontext.service import build_server


def write_text_pdf(path: Path, pages: list[str]) -> None:
    """Write a tiny dependency-free PDF fixture with one Helvetica text line per page."""
    font_id = 3 + len(pages) * 2
    page_ids = [3 + index * 2 for index in range(len(pages))]
    content_ids = [page_id + 1 for page_id in page_ids]
    objects: dict[int, bytes] = {
        1: b"<< /Type /Catalog /Pages 2 0 R >>",
        2: (
            f"<< /Type /Pages /Kids [{' '.join(f'{item} 0 R' for item in page_ids)}] "
            f"/Count {len(page_ids)} >>"
        ).encode("ascii"),
        font_id: b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    }
    for page_id, content_id, text in zip(page_ids, content_ids, pages, strict=True):
        escaped = text.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")
        stream = f"BT /F1 18 Tf 72 720 Td ({escaped}) Tj ET".encode("latin-1")
        objects[page_id] = (
            f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            f"/Resources << /Font << /F1 {font_id} 0 R >> >> /Contents {content_id} 0 R >>"
        ).encode("ascii")
        objects[content_id] = (
            f"<< /Length {len(stream)} >>\nstream\n".encode("ascii")
            + stream
            + b"\nendstream"
        )
    payload = bytearray(b"%PDF-1.4\n")
    offsets = [0] * (font_id + 1)
    for object_id in range(1, font_id + 1):
        offsets[object_id] = len(payload)
        payload.extend(f"{object_id} 0 obj\n".encode("ascii"))
        payload.extend(objects[object_id])
        payload.extend(b"\nendobj\n")
    xref = len(payload)
    payload.extend(f"xref\n0 {font_id + 1}\n".encode("ascii"))
    payload.extend(b"0000000000 65535 f \n")
    for object_id in range(1, font_id + 1):
        payload.extend(f"{offsets[object_id]:010d} 00000 n \n".encode("ascii"))
    payload.extend(
        f"trailer\n<< /Size {font_id + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode("ascii")
    )
    path.write_bytes(payload)


def configured_workspace(tmp_path: Path):
    root = tmp_path / "research-project"
    (root / "docs").mkdir(parents=True)
    (root / "src").mkdir()
    (root / "runs" / "baseline").mkdir(parents=True)
    (root / "runs" / "treatment").mkdir(parents=True)
    research = root / "research" / "idea_runs" / "study-1"
    research.mkdir(parents=True)
    (root / "docs" / "context.yaml").write_text("objective: portable research context\n")
    (root / "src" / "runner.py").write_text("def executable_endpoint():\n    return 'ok'\n")
    (root / "runs" / "baseline" / "summary.json").write_text(json.dumps({
        "run_label": "baseline", "n": 20, "accuracy": 0.4, "config": {"seed": 1},
    }))
    (root / "runs" / "treatment" / "summary.json").write_text(json.dumps({
        "run_label": "treatment", "n": 20, "accuracy": 0.55, "config": {"seed": 1},
    }))
    (research / "run_state.json").write_text(json.dumps({
        "run_id": "study-1", "status": "active", "phase": "PILOT",
        "pending_decision": "run the confirmatory experiment",
    }))
    (research / "research_contract.json").write_text(json.dumps({
        "goal": {"objective": "test a mechanism"}, "claim": {"claim_id": "C1"},
    }))
    (research / "claim_evidence.json").write_text(json.dumps({
        "claims": [{"claim_id": "C1", "status": "pilot"}],
        "evidence": [{"evidence_id": "EV-1"}], "causal_chain": [],
    }))
    (research / "decision.json").write_text(json.dumps({
        "outcome": "UNDECIDED", "unresolved_risks": ["small sample"],
    }))
    config_path = tmp_path / "labcontext.toml"
    config_path.write_text(f'''[server]
host="127.0.0.1"
port=1455
path="/mcp"
[project]
id="test"
root="{root}"
allowed_roots=["."]
denied_globs=["**/.env"]
[registry]
default_workspace="main"
allowed_roots=["{tmp_path}"]
[limits]
max_response_bytes=12000
[storage]
database="{tmp_path / 'state' / 'index.db'}"
manifest_dir="{tmp_path / 'state' / 'manifests'}"
ingest_roots=[]
[workspaces.main]
name="Research Project"
root="."
context_file="docs/context.yaml"
aliases=["project"]
adapters=["generic", "generic_experiments", "research_dossier"]
[[workspaces.main.assets]]
id="docs"
kind="project_docs"
include=["docs/**/*"]
adapter="markdown"
authority="canonical_fact"
index_content="text"
[[workspaces.main.assets]]
id="code"
kind="source_code"
include=["src/**/*"]
adapter="generic_code"
authority="observed_artifact"
index_content="text"
[[workspaces.main.assets]]
id="runs"
kind="experiment_run"
include=["runs/*/summary.json"]
adapter="generic_experiments"
authority="observed_artifact"
index_content="metrics"
[[workspaces.main.assets]]
id="research"
kind="research_state"
include=["research/idea_runs/*/*.json"]
adapter="research_dossier"
authority="canonical_fact"
index_content="structured"
[summarizer]
command=["echo"]
timeout_seconds=1
max_context_bytes=1000
session_dir="{tmp_path / 'sessions'}"
max_session_chars=1000
max_session_messages=10
''')
    return load_config(config_path)


def test_workspace_overview_and_research_adapter(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    listing = list_workspaces_data(config)
    assert listing["default_workspace_id"] == "main"
    assert "research_context" in listing["workspaces"][0]["capabilities"]
    overview = workspace_overview_data(config)
    assert overview["resolved_workspace_id"] == "main"
    assert len(overview["recent_experiments"]) == 2
    context = research_context_data(config, research_id="active")
    assert context["phase"] == "PILOT"
    assert context["decision"]["outcome"] == "UNDECIDED"
    assert context["evidence_refs"]


def test_workspace_overview_exposes_three_bounded_recent_session_digests(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    sessions = config.summarizer_session_dir
    sessions.mkdir(parents=True)
    now = time.time()
    for index in range(4):
        path = sessions / f"session-{index}.jsonl"
        path.write_text("\n".join([
            json.dumps({
                "type": "session_meta",
                "payload": {
                    "cwd": str(config.workspaces["main"].root),
                    "session_id": f"session-{index}",
                    "timestamp": f"2026-08-1{index}T00:00:00Z",
                },
            }),
            json.dumps({
                "type": "response_item",
                "payload": {"type": "message", "role": "user", "content": [{
                    "type": "input_text", "text": f"Investigate experiment {index} token=sk-abcdefghijklmnop",
                }]},
            }),
            json.dumps({
                "type": "response_item",
                "payload": {"type": "message", "role": "assistant", "content": [{
                    "type": "output_text", "text": f"Checked experiment {index} and recorded the result.",
                }]},
            }),
        ]) + "\n", encoding="utf-8")
        os.utime(path, (now + index, now + index))

    overview = workspace_overview_data(config)
    working = overview["working_context"]
    assert working["authority"] == "unverified_session_digest"
    assert [item["session_id"] for item in working["sessions"]] == [
        "session-3", "session-2", "session-1",
    ]
    assert working["coverage"]["max_sessions"] == 3
    serialized = json.dumps(working, ensure_ascii=False)
    assert len(serialized.encode()) < 4200
    assert "sk-abcdefghijklmnop" not in serialized
    assert "[REDACTED]" in serialized
    assert "excerpt" not in working["sessions"][0]


def test_search_and_bounded_evidence_read(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    found = search_evidence_data(config, "executable_endpoint", scopes=["code"])
    assert found["results"][0]["path"] == "src/runner.py"
    evidence = get_evidence_data(config, [found["results"][0]["evidence_ref"]])
    assert evidence["results"][0]["status"] == "ok"
    assert "executable_endpoint" in evidence["results"][0]["content"]


def test_inspect_exact_unindexed_file_with_bounded_views(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    root = config.workspaces["main"].root
    analysis = root / "research" / "idea_runs" / "study-1" / "artifacts" / "analysis"
    analysis.mkdir(parents=True)
    dossier = analysis.parent.parent / "IDEA_DOSSIER.md"
    dossier.write_text("# Core idea\n\n## Decision\n\nPIVOT to a stage-local timing claim.\n", encoding="utf-8")
    result = analysis / "endpoint.json"
    result.write_text(json.dumps({
        "models": {"qwen": {"rd_uc": 0.241}, "llava": {"rd_uc": 0.165}},
        "decision": "PIVOT",
    }), encoding="utf-8")

    outline = inspect_file_data(config, str(dossier), "main", view="outline")
    assert outline["configured_asset"] is None
    assert [item["title"] for item in outline["outline"]["headings"]] == ["Core idea", "Decision"]
    searched = inspect_file_data(config, dossier.relative_to(root).as_posix(), "main", view="search", query="PIVOT")
    assert searched["matches"][0]["evidence_ref"].startswith("ev_")
    selected = inspect_file_data(
        config, result.relative_to(root).as_posix(), "main", view="json_pointer",
        json_pointer="/models/llava",
    )
    assert '\"rd_uc\": 0.165' in selected["content"]
    lines = inspect_file_data(config, dossier.relative_to(root).as_posix(), "main", view="lines", start_line=1, end_line=3)
    assert lines["location"] == {"start_line": 1, "end_line": 3}


def test_inspect_file_rejects_outside_denied_and_binary_paths(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    root = config.workspaces["main"].root
    (root / ".env.local").write_text("SECRET=value\n", encoding="utf-8")
    (root / "blob.bin").write_bytes(b"\x00\x01")
    outside = tmp_path / "outside.md"
    outside.write_text("outside\n", encoding="utf-8")
    for path in (root / ".env.local", root / "blob.bin", outside):
        try:
            inspect_file_data(config, str(path), "main")
        except ValueError:
            pass
        else:
            raise AssertionError(f"unsafe path was inspectable: {path}")


def test_inspect_path_reads_unregistered_files_and_directories(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    unregistered = tmp_path / "unregistered-project"
    (unregistered / "src").mkdir(parents=True)
    readme = unregistered / "README.md"
    readme.write_text("# Unregistered\n\nDirect path content.\n", encoding="utf-8")
    (unregistered / "src" / "main.py").write_text("def main():\n    return 1\n", encoding="utf-8")

    tree = inspect_path_data(config, str(unregistered), depth=2)
    assert tree["scope"] == "direct_path"
    assert tree["path_type"] == "directory"
    assert {item["path"] for item in tree["entries"]} >= {"README.md", "src", "src/main.py"}
    content = inspect_path_data(config, str(readme))
    assert content["view"] == "full_bounded"
    assert "Direct path content" in content["content"]


def test_inspect_path_extracts_safe_html_without_active_content(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    document = tmp_path / "paper.html"
    document.write_text(
        """<!doctype html><html><head><title>Evidence Inventory</title>
        <style>.hidden { display: none }</style><script>SECRET_SCRIPT_TEXT</script></head>
        <body><h1>Training evidence</h1><p>Visible repair result.</p>
        <h2>Limitations</h2><p>Public evaluation is still required.</p></body></html>""",
        encoding="utf-8",
    )
    content = inspect_path_data(config, str(document))
    assert content["file"]["reader"] == "html"
    assert content["document"]["title"] == "Evidence Inventory"
    assert "Visible repair result" in content["content"]
    assert "SECRET_SCRIPT_TEXT" not in content["content"]
    outline = inspect_path_data(config, str(document), view="outline")
    assert [item["title"] for item in outline["outline"]["headings"]] == [
        "Training evidence", "Limitations",
    ]
    searched = inspect_path_data(config, str(document), view="search", query="Public evaluation")
    assert searched["match_count_returned"] == 1


def test_inspect_path_extracts_and_pages_text_pdf(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    document = tmp_path / "paper.pdf"
    write_text_pdf(document, ["TADrop model merging", "Second page experiment", "Final conclusion"])
    content = inspect_path_data(
        config, str(document), view="pages", start_page=2, end_page=3,
    )
    assert content["file"]["reader"] == "pdf"
    assert content["document"]["page_count"] == 3
    assert content["location"] == {"start_page": 2, "end_page": 3}
    assert "Second page experiment" in content["content"]
    assert "Final conclusion" in content["content"]
    searched = inspect_path_data(config, str(document), view="search", query="model merging")
    assert searched["matches"][0]["page"] == 1
    assert "TADrop" in searched["matches"][0]["content"]


def test_inspect_path_marks_document_readers_in_directory_tree(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    (tmp_path / "paper.html").write_text("<h1>Paper</h1>", encoding="utf-8")
    write_text_pdf(tmp_path / "paper.pdf", ["Readable PDF"])
    tree = inspect_path_data(config, str(tmp_path), depth=1)
    entries = {item["path"]: item for item in tree["entries"]}
    assert entries["paper.html"]["reader"] == "html"
    assert entries["paper.pdf"]["reader"] == "pdf"
    assert entries["paper.pdf"]["text_readable"] is True


def test_inspect_path_requires_absolute_allowed_safe_path(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    outside = tmp_path.parent / "outside-direct-path.md"
    outside.write_text("outside\n", encoding="utf-8")
    secret = tmp_path / ".env.local"
    secret.write_text("TOKEN=value\n", encoding="utf-8")
    binary = tmp_path / "blob.bin"
    binary.write_bytes(b"\x00\x01")
    for path in ("relative.md", str(outside), str(secret), str(binary)):
        try:
            inspect_path_data(config, path)
        except ValueError:
            pass
        else:
            raise AssertionError(f"unsafe direct path was inspectable: {path}")


def test_experiment_query_and_compare(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    workspace = resolve_workspace(config)
    queried = query_experiments(config, workspace, metrics=["accuracy", "n"], detail="metrics")
    assert {item["experiment_id"] for item in queried["experiments"]} == {"baseline", "treatment"}
    compared = compare_experiments(
        config, workspace, ["baseline", "treatment"], metrics=["accuracy", "n"],
        include_config_diff=True,
    )
    assert compared["absolute_deltas_from_baseline"]["accuracy"]["treatment"] == 0.15000000000000002
    assert compared["comparability_warnings"] == []


def test_public_mcp_surface_is_the_eleven_general_tools(tmp_path: Path) -> None:
    config = configured_workspace(tmp_path)
    server = build_server(config)
    tools = asyncio.run(server.get_tools())
    assert set(tools) == {
        "list_workspaces", "workspace_overview", "research_context",
        "search_evidence", "get_evidence", "inspect_file", "inspect_path", "query_experiments",
        "compare_experiments", "request_analysis", "get_job",
    }
    schema = tools["inspect_path"].parameters
    assert {"start_page", "end_page"} <= set(schema["properties"])
