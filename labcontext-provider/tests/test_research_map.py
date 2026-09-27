import json
from pathlib import Path

import pytest

from labcontext.config import load_config
from labcontext.research_map import (
    ResearchMapConflict, ResearchMapError, apply_map_proposal, apply_map_patch,
    codex_map_context, create_map_proposal, focus_capsule, initialize_research_map,
    list_map_proposals, load_research_map, map_paths, reject_map_proposal,
    research_map_bundle, review_with_latest_session, save_layout,
)


def configured_map(tmp_path: Path):
    workspace = tmp_path / "project"
    (workspace / ".labcontext").mkdir(parents=True)
    (workspace / ".labcontext" / "context.yaml").write_text(
        "schema_version: 1\nname: Project\noverview: Test a causal research mechanism.\n"
        "current_stage: Pilot runner repaired.\nopen_questions:\n"
        "  - Does the effect survive a held-out test?\n  - Is the harness an alternative explanation?\n"
        "authority: user_reviewed\n",
        encoding="utf-8",
    )
    config_path = tmp_path / "labcontext.toml"
    config_path.write_text(f'''[server]
host="127.0.0.1"
port=1455
path="/mcp"
[project]
id="test"
root="{workspace}"
allowed_roots=["."]
denied_globs=[]
[registry]
default_workspace="main"
allowed_roots=["{tmp_path}"]
[limits]
max_response_bytes=20000
[storage]
database="{tmp_path / 'state' / 'index.db'}"
manifest_dir="{tmp_path / 'state' / 'manifests'}"
ingest_roots=[]
[workspaces.main]
name="Project"
root="."
context_file=".labcontext/context.yaml"
adapters=["generic"]
[summarizer]
command=["echo"]
timeout_seconds=1
max_context_bytes=1000
session_dir="{tmp_path / 'sessions'}"
''', encoding="utf-8")
    return load_config(config_path)


def test_initialize_and_focus_projection(tmp_path: Path) -> None:
    config = configured_map(tmp_path)
    initialized = initialize_research_map(config, "main")
    assert initialized["revision"] == 1
    value = initialized["research_map"]
    assert value["review_state"] == "needs_review"
    assert {node["type"] for node in value["nodes"]} >= {"core_idea", "current_target"}
    capsule = focus_capsule(value)
    assert capsule["core_idea"]["title"] == "Test a causal research mechanism."
    assert capsule["current_target"] == {
        "id": "target:current", "title": "Does the effect survive a held-out test?", "status": "active",
    }
    assert any(node["type"] == "branch" for node in value["nodes"])
    assert map_paths(config.workspaces["main"])["events"].is_file()


def test_patch_is_revisioned_and_codex_cannot_change_locked_fields(tmp_path: Path) -> None:
    config = configured_map(tmp_path)
    initialize_research_map(config, "main")
    patch = {
        "base_revision": 1,
        "summary": "Add one falsifiable branch",
        "operations": [
            {"op": "add_node", "node": {
                "id": "branch:harness", "type": "branch", "title": "Harness explanation",
                "status": "active",
            }},
            {"op": "add_edge", "edge": {
                "source": "idea:main", "target": "branch:harness", "relation": "decomposes_into",
            }},
            {"op": "set_current_focus", "node_id": "branch:harness"},
        ],
    }
    result = apply_map_patch(config, "main", patch, actor="user")
    assert result["revision"] == 2
    assert result["research_map"]["current_focus_node_id"] == "branch:harness"
    with pytest.raises(ResearchMapConflict, match="revision_conflict"):
        apply_map_patch(config, "main", patch, actor="user")

    lock_patch = {
        "base_revision": 2,
        "operations": [{"op": "update_node", "node_id": "branch:harness", "changes": {
            "locked_fields": ["title"],
        }}],
    }
    locked = apply_map_patch(config, "main", lock_patch, actor="user")
    with pytest.raises(ResearchMapError, match="protected fields"):
        apply_map_patch(config, "main", {
            "base_revision": locked["revision"],
            "operations": [{"op": "update_node", "node_id": "branch:harness", "changes": {
                "title": "Overwritten by Codex",
            }}],
        }, actor="user", proposal_source="session-1")


def test_archive_layout_and_codex_context_are_separate(tmp_path: Path) -> None:
    config = configured_map(tmp_path)
    initialize_research_map(config, "main")
    archived = apply_map_patch(config, "main", {
        "base_revision": 1,
        "operations": [{"op": "archive_node", "node_id": "target:current"}],
    }, actor="user")
    assert next(node for node in archived["research_map"]["nodes"] if node["id"] == "target:current")["status"] == "archived"
    layout = save_layout(config, "main", {
        "nodes": [{"id": "idea:main", "x": 12.345, "y": 67.891}, {"id": "unknown", "x": 0, "y": 0}],
        "viewport": {"x": 1, "y": 2, "zoom": 1.2},
    })
    assert layout["layout"]["nodes"] == [{"id": "idea:main", "x": 12.35, "y": 67.89, "collapsed": False}]
    bundle = research_map_bundle(config, "main")
    assert bundle["research_map"]["revision"] == 2
    assert bundle["layout"]["viewport"]["zoom"] == 1.2
    assert [event["revision"] for event in bundle["events"]] == [2, 1]
    context = codex_map_context(config, "main")
    assert "viewport" not in json.dumps(context)
    assert context["update_protocol"]["format"] == "domain_map_patch_v1"


def test_proposal_lifecycle(tmp_path: Path) -> None:
    config = configured_map(tmp_path)
    initialize_research_map(config, "main")
    proposal = create_map_proposal(config, "main", {
        "base_revision": 1,
        "summary": "Add a candidate branch",
        "operations": [{"op": "add_node", "node": {
            "id": "branch:candidate", "type": "branch", "title": "Candidate mechanism",
        }}],
    }, source_kind="codex_session", source_session_id="session-1")
    assert proposal["status"] == "pending"
    assert list_map_proposals(config, "main")["proposals"][0]["proposal_id"] == proposal["proposal_id"]
    applied = apply_map_proposal(config, "main", proposal["proposal_id"])
    assert applied["proposal_status"] == "applied"
    assert "branch:candidate" in {node["id"] for node in load_research_map(config, "main")["nodes"]}

    second = create_map_proposal(config, "main", {
        "base_revision": 2,
        "summary": "Another proposal",
        "operations": [{"op": "set_current_focus", "node_id": "idea:main"}],
    }, source_kind="codex_session")
    assert reject_map_proposal(config, "main", second["proposal_id"])["status"] == "rejected"


def test_review_queues_latest_exact_workspace_session(tmp_path: Path, monkeypatch) -> None:
    config = configured_map(tmp_path)
    initialize_research_map(config, "main")
    sessions = config.summarizer_session_dir
    sessions.mkdir(parents=True)
    session = sessions / "session.jsonl"
    session.write_text(json.dumps({
        "type": "session_meta", "payload": {
            "cwd": str(config.workspaces["main"].root), "session_id": "session-exact",
            "timestamp": "2026-08-23T00:00:00Z",
        },
    }) + "\n", encoding="utf-8")
    calls = []
    class Result:
        returncode = 0
        stdout = ""
        stderr = ""
    monkeypatch.setattr("labcontext.research_map.subprocess.run", lambda command, **kwargs: calls.append(command) or Result())
    result = review_with_latest_session(config, "main")
    assert result["status"] == "queued_to_session"
    assert result["session"]["session_id"] == "session-exact"
    assert calls[0][1:4] == ["queue", "--thread", "session-exact"]
