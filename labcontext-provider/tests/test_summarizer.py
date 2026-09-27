import json
from pathlib import Path
import sys
import time

import labcontext.summarizer as summarizer_module
from labcontext.config import load_config
from labcontext.summarizer import build_evidence, get_job, get_summary, recover_interrupted_jobs, request_summary


def test_overview_progress_file_does_not_change_evidence_fingerprint(tmp_path: Path) -> None:
    project = tmp_path / "project"
    workspace = project / "hvs5"
    (workspace / "docs").mkdir(parents=True)
    config_path = tmp_path / "labcontext.toml"
    config_path.write_text(f'''[server]
host="127.0.0.1"
port=1455
path="/mcp"
[project]
id="test"
root="{project}"
allowed_roots=["hvs5"]
denied_globs=[]
[limits]
max_response_bytes=12000
[storage]
database="{tmp_path / 'state' / 'index.db'}"
manifest_dir="{tmp_path / 'state' / 'manifests'}"
ingest_roots=[]
[workspaces.hvs5]
root="hvs5"
context_file="docs/labcontext-context.yaml"
[summarizer]
command=["echo"]
timeout_seconds=1
max_context_bytes=1000
session_dir="{tmp_path / 'sessions'}"
''')
    config = load_config(config_path)
    _, before = build_evidence(config, "hvs5")
    (workspace / "docs" / "labcontext-context.yaml").write_text(
        "schema_version: 1\nname: HVS\noverview: provisional\n"
        "generation:\n  status: running\n  job_id: ws_test\n",
        encoding="utf-8",
    )
    _, after = build_evidence(config, "hvs5")
    assert after == before


def test_summary_worker_uses_registered_workspace(tmp_path: Path) -> None:
    project = tmp_path / "project"
    workspace = project / "hvs5"
    (workspace / "logs").mkdir(parents=True)
    (workspace / "docs").mkdir()
    (workspace / "docs" / "labcontext-context.yaml").write_text("objective: test\n")
    worker = tmp_path / "worker.py"
    worker.write_text('''import json, pathlib, sys
assert sys.argv[sys.argv.index("--model") + 1] == "gpt-5.6-terra"
assert sys.argv[sys.argv.index("--config") + 1] == 'model_reasoning_effort="medium"'
assert "<workspace_evidence>" in sys.stdin.read()
assert "--json" in sys.argv
out = pathlib.Path(sys.argv[sys.argv.index("--output-last-message") + 1])
out.write_text(json.dumps({"objective":"test","current_status":"ok","latest_codex_session":{"status":"not_found","session_id":"","updated_at":"","workspace_match":"","summary":"","recent_actions":[]},"recent_experiment_facts":[],"open_questions":[],"suggested_next_actions":[],"evidence_refs":[]}))
''')
    config_path = tmp_path / "labcontext.toml"
    config_path.write_text(f'''[server]
host="127.0.0.1"
port=1455
path="/mcp"
[project]
id="test"
root="{project}"
allowed_roots=["hvs5"]
denied_globs=[]
[limits]
max_response_bytes=12
[storage]
database="{tmp_path / 'state' / 'index.db'}"
manifest_dir="{tmp_path / 'state' / 'manifests'}"
ingest_roots=["hvs5/logs"]
[workspaces.hvs5]
root="hvs5"
context_file="docs/labcontext-context.yaml"
[summarizer]
command=["{sys.executable}", "{worker}"]
timeout_seconds=5
max_context_bytes=100
session_dir="{tmp_path / 'sessions'}"
max_session_chars=1000
max_session_messages=10
''')
    config = load_config(config_path)
    requested = request_summary(config, "hvs5")
    assert requested["status"] == "running"
    assert requested["progress"] == "queued"
    for _ in range(20):
        result = get_summary(config, "hvs5")
        if result["status"] != "running":
            break
        time.sleep(0.05)
    assert result["status"] == "ready"
    assert result["progress"] == "completed"
    assert result["summary"]["objective"] == "test"


def test_summary_includes_latest_matching_codex_session(tmp_path: Path) -> None:
    project = tmp_path / "project"
    workspace = project / "hvs5"
    workspace.mkdir(parents=True)
    sessions = tmp_path / "sessions" / "2026" / "08" / "19"
    sessions.mkdir(parents=True)
    session = sessions / "rollout.jsonl"
    session.write_text("\n".join([
        json.dumps({"type": "session_meta", "payload": {"cwd": str(workspace), "session_id": "session-1", "timestamp": "2026-08-19T00:00:00Z"}}),
        json.dumps({"type": "response_item", "payload": {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "Inspect the runner"}]}}),
        json.dumps({"type": "response_item", "payload": {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "Changed runner; token=sk-abcdefghijklmnop"}]}}),
    ]) + "\n")
    worker = tmp_path / "worker.py"
    worker.write_text('''import json, pathlib, sys
prompt = sys.stdin.read()
assert "registered workspace" in prompt
out = pathlib.Path(sys.argv[sys.argv.index("--output-last-message") + 1])
out.write_text(json.dumps({"objective":"test","current_status":"ok","latest_codex_session":{"status":"found","session_id":"session-1","updated_at":"2026-08-19T00:00:00Z","workspace_match":"exact_cwd","summary":"runner changed","recent_actions":["inspect runner"]},"recent_experiment_facts":[],"open_questions":[],"suggested_next_actions":[],"evidence_refs":["Codex session session-1"]}))
''')
    config_path = tmp_path / "labcontext.toml"
    config_path.write_text(f'''[server]
host="127.0.0.1"
port=1455
path="/mcp"
[project]
id="test"
root="{project}"
allowed_roots=["hvs5"]
denied_globs=[]
[limits]
max_response_bytes=12
[storage]
database="{tmp_path / 'state' / 'index.db'}"
manifest_dir="{tmp_path / 'state' / 'manifests'}"
ingest_roots=[]
[workspaces.hvs5]
root="hvs5"
context_file="context.yaml"
[summarizer]
command=["{sys.executable}", "{worker}"]
timeout_seconds=5
max_context_bytes=100
session_dir="{tmp_path / 'sessions'}"
max_session_chars=1000
max_session_messages=10
    ''')
    config = load_config(config_path)
    evidence, _ = build_evidence(config, "hvs5")
    session_evidence = evidence["latest_codex_session"]
    assert session_evidence["session_id"] == "session-1"
    assert "sk-abcdefghijklmnop" not in session_evidence["excerpt"]
    assert "[REDACTED]" in session_evidence["excerpt"]
    assert request_summary(config, "hvs5")["status"] == "running"
    for _ in range(20):
        result = get_summary(config, "hvs5")
        if result["status"] != "running":
            break
        time.sleep(0.05)
    assert result["status"] == "ready"
    assert result["summary"]["latest_codex_session"]["session_id"] == "session-1"


def test_summary_uses_request_evidence_snapshot_while_session_keeps_writing(
    tmp_path: Path, monkeypatch,
) -> None:
    project = tmp_path / "project"
    workspace = project / "hvs5"
    workspace.mkdir(parents=True)
    sessions = tmp_path / "sessions" / "2026" / "08" / "19"
    sessions.mkdir(parents=True)
    session = sessions / "rollout.jsonl"
    session.write_text("\n".join([
        json.dumps({"type": "session_meta", "payload": {
            "cwd": str(workspace), "session_id": "session-1",
            "timestamp": "2026-08-19T00:00:00Z",
        }}),
        json.dumps({"type": "response_item", "payload": {
            "type": "message", "role": "user",
            "content": [{"type": "input_text", "text": "Inspect the runner"}],
        }}),
    ]) + "\n", encoding="utf-8")
    worker = tmp_path / "worker.py"
    worker.write_text('''import json, pathlib, sys
assert "Inspect the runner" in sys.stdin.read()
out = pathlib.Path(sys.argv[sys.argv.index("--output-last-message") + 1])
out.write_text(json.dumps({"objective":"test","current_status":"ok","latest_codex_session":{"status":"found","session_id":"session-1","updated_at":"2026-08-19T00:00:00Z","workspace_match":"exact_cwd","summary":"runner inspection","recent_actions":[]},"recent_experiment_facts":[],"open_questions":[],"suggested_next_actions":[],"evidence_refs":["Codex session session-1"]}))
''', encoding="utf-8")
    config_path = tmp_path / "labcontext.toml"
    config_path.write_text(f'''[server]
host="127.0.0.1"
port=1455
path="/mcp"
[project]
id="test"
root="{project}"
allowed_roots=["hvs5"]
denied_globs=[]
[limits]
max_response_bytes=12
[storage]
database="{tmp_path / 'state' / 'index.db'}"
manifest_dir="{tmp_path / 'state' / 'manifests'}"
ingest_roots=[]
[workspaces.hvs5]
root="hvs5"
context_file="context.yaml"
[summarizer]
command=["{sys.executable}", "{worker}"]
timeout_seconds=5
max_context_bytes=100
session_dir="{tmp_path / 'sessions'}"
max_session_chars=1000
max_session_messages=10
''', encoding="utf-8")
    original_run_job = summarizer_module._run_job

    def append_session_event_then_run(*args, **kwargs) -> None:
        with session.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps({
                "timestamp": "2026-08-19T00:00:01Z",
                "type": "response_item",
                "payload": {"type": "reasoning", "summary": []},
            }) + "\n")
        original_run_job(*args, **kwargs)

    monkeypatch.setattr(summarizer_module, "_run_job", append_session_event_then_run)
    config = load_config(config_path)
    assert request_summary(config, "hvs5")["status"] == "running"
    for _ in range(40):
        result = get_summary(config, "hvs5")
        if result["status"] != "running":
            break
        time.sleep(0.05)
    assert result["status"] == "ready"
    assert result["summary"]["objective"] == "test"


def test_summary_timeout_is_reported_structurally(tmp_path: Path) -> None:
    project = tmp_path / "project"
    (project / "hvs5").mkdir(parents=True)
    worker = tmp_path / "worker.py"
    worker.write_text("import time\ntime.sleep(3)\n")
    config_path = tmp_path / "labcontext.toml"
    config_path.write_text(f'''[server]
host="127.0.0.1"
port=1455
path="/mcp"
[project]
id="test"
root="{project}"
allowed_roots=["hvs5"]
denied_globs=[]
[limits]
max_response_bytes=12
[storage]
database="{tmp_path / 'state' / 'index.db'}"
manifest_dir="{tmp_path / 'state' / 'manifests'}"
ingest_roots=[]
[workspaces.hvs5]
root="hvs5"
context_file="context.yaml"
[summarizer]
command=["{sys.executable}", "{worker}"]
timeout_seconds=1
max_context_bytes=100
session_dir="{tmp_path / 'sessions'}"
max_session_chars=1000
max_session_messages=10
''')
    config = load_config(config_path)
    request_summary(config, "hvs5")
    for _ in range(40):
        result = get_summary(config, "hvs5")
        if result["status"] != "running":
            break
        time.sleep(0.05)
    assert result["status"] == "failed"
    assert result["error"]["error_type"] == "timeout"
    assert result["error"]["timeout_seconds"] == 1


def test_summary_worker_drains_large_stderr_without_deadlock(tmp_path: Path) -> None:
    project = tmp_path / "project"
    (project / "hvs5").mkdir(parents=True)
    worker = tmp_path / "worker.py"
    worker.write_text('''import json, pathlib, sys
sys.stdin.read()
sys.stderr.write("x" * 200_000)
sys.stderr.flush()
print(json.dumps({"type":"thread.started","thread_id":"test-thread"}), flush=True)
print(json.dumps({"type":"turn.started"}), flush=True)
print(json.dumps({"type":"item.started","item":{"type":"command_execution"}}), flush=True)
print(json.dumps({"type":"item.completed","item":{"type":"agent_message","text":"done"}}), flush=True)
print(json.dumps({"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":1}}), flush=True)
out = pathlib.Path(sys.argv[sys.argv.index("--output-last-message") + 1])
out.write_text(json.dumps({"objective":"test","current_status":"ok","latest_codex_session":{"status":"not_found","session_id":"","updated_at":"","workspace_match":"","summary":"","recent_actions":[]},"recent_experiment_facts":[],"open_questions":[],"suggested_next_actions":[],"evidence_refs":[]}))
''')
    config_path = tmp_path / "labcontext.toml"
    config_path.write_text(f'''[server]
host="127.0.0.1"
port=1455
path="/mcp"
[project]
id="test"
root="{project}"
allowed_roots=["hvs5"]
denied_globs=[]
[limits]
max_response_bytes=12000
[storage]
database="{tmp_path / 'state' / 'index.db'}"
manifest_dir="{tmp_path / 'state' / 'manifests'}"
ingest_roots=[]
[workspaces.hvs5]
root="hvs5"
context_file="context.yaml"
[summarizer]
command=["{sys.executable}", "{worker}"]
timeout_seconds=5
max_context_bytes=100
session_dir="{tmp_path / 'sessions'}"
max_session_chars=1000
max_session_messages=10
''')
    config = load_config(config_path)
    job_id = request_summary(config, "hvs5")["job_id"]
    for _ in range(40):
        result = get_job(config, job_id)
        if result["status"] != "running":
            break
        time.sleep(0.05)
    assert result["status"] == "completed"
    assert result["progress"] == "completed"
    diagnostics = result["result"]["analysis_diagnostics"]
    assert diagnostics["stderr_bytes"] > 65_536
    assert diagnostics["event_count"] == 5
    assert diagnostics["workspace_reads"] == 1


def test_service_recovery_marks_running_job_interrupted(tmp_path: Path) -> None:
    project = tmp_path / "project"
    (project / "hvs5").mkdir(parents=True)
    config_path = tmp_path / "labcontext.toml"
    config_path.write_text(f'''[server]
host="127.0.0.1"
port=1455
path="/mcp"
[project]
id="test"
root="{project}"
allowed_roots=["hvs5"]
denied_globs=[]
[limits]
max_response_bytes=12
[storage]
database="{tmp_path / 'state' / 'index.db'}"
manifest_dir="{tmp_path / 'state' / 'manifests'}"
ingest_roots=[]
[workspaces.hvs5]
root="hvs5"
context_file="context.yaml"
[summarizer]
command=["echo"]
timeout_seconds=1
max_context_bytes=100
session_dir="{tmp_path / 'sessions'}"
max_session_chars=1000
max_session_messages=10
''')
    config = load_config(config_path)
    import sqlite3
    from labcontext.index import _connect
    from labcontext.summarizer import _ensure_tables
    _ensure_tables(config)
    with _connect(config) as connection:
        connection.execute("""INSERT INTO workspace_summaries(workspace_id,fingerprint,status,job_id,started_at,updated_at,progress)
                            VALUES('hvs5','fingerprint','running','job-1','start','start','Codex is summarizing')""")
    assert recover_interrupted_jobs(config) == 1
    result = get_summary(config, "hvs5")
    assert result["status"] == "interrupted"
    assert result["progress"] == "interrupted: service restarted"
    assert result["error"]["error_type"] == "service_restarted"


def test_different_analysis_questions_keep_independent_jobs(tmp_path: Path) -> None:
    project = tmp_path / "project"
    (project / "hvs5").mkdir(parents=True)
    worker = tmp_path / "worker.py"
    worker.write_text('''import json, pathlib, sys
out = pathlib.Path(sys.argv[sys.argv.index("--output-last-message") + 1])
out.write_text(json.dumps({"objective":"test","current_status":"ok","latest_codex_session":{"status":"not_found","session_id":"","updated_at":"","workspace_match":"","summary":"","recent_actions":[]},"recent_experiment_facts":[],"open_questions":[],"suggested_next_actions":[],"evidence_refs":[]}))
''')
    config_path = tmp_path / "labcontext.toml"
    config_path.write_text(f'''[server]
host="127.0.0.1"
port=1455
path="/mcp"
[project]
id="test"
root="{project}"
allowed_roots=["hvs5"]
denied_globs=[]
[limits]
max_response_bytes=12000
[storage]
database="{tmp_path / 'state' / 'index.db'}"
manifest_dir="{tmp_path / 'state' / 'manifests'}"
ingest_roots=[]
[workspaces.hvs5]
root="hvs5"
context_file="context.yaml"
[summarizer]
command=["{sys.executable}", "{worker}"]
timeout_seconds=5
max_context_bytes=100
session_dir="{tmp_path / 'sessions'}"
max_session_chars=1000
max_session_messages=10
''')
    config = load_config(config_path)
    first = request_summary(config, "hvs5", question="What is the first question?")
    second = request_summary(config, "hvs5", question="What is the second question?")
    assert first["job_id"] != second["job_id"]
    for job_id in (first["job_id"], second["job_id"]):
        for _ in range(40):
            result = get_job(config, job_id)
            if result["status"] != "running":
                break
            time.sleep(0.05)
        assert result["status"] == "completed"
        assert result["job_id"] == job_id
