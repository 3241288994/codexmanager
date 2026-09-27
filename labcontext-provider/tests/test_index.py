import json
from pathlib import Path

import pytest

from labcontext.config import load_config
from labcontext.index import get_run, ingest_run, search_runs


def config_for(tmp_path: Path):
    project = tmp_path / "project"
    run = project / "hvs5" / "logs" / "demo_run"
    run.mkdir(parents=True)
    (run / "summary.json").write_text(json.dumps({
        "run_label": "demo evaluation", "n": 10, "baseline_accuracy": 0.2,
        "revised_accuracy": 0.3, "config": {"seed": 42},
    }))
    config_path = tmp_path / "labcontext.toml"
    config_path.write_text(f'''[server]\nhost = "127.0.0.1"\nport = 1455\npath = "/mcp"\n\n[project]\nid = "test"\nroot = "{project}"\nallowed_roots = ["hvs5"]\ndenied_globs = ["**/.env"]\n\n[limits]\nmax_response_bytes = 12\n\n[storage]\ndatabase = "{tmp_path / 'state' / 'index.db'}"\nmanifest_dir = "{tmp_path / 'state' / 'manifests'}"\ningest_roots = ["hvs5/logs"]\n\n[summarizer]\ncommand = ["echo"]\ntimeout_seconds = 1\nmax_context_bytes = 100\n''')
    return load_config(config_path), run


def test_ingest_search_and_get_run(tmp_path: Path) -> None:
    config, run = config_for(tmp_path)
    manifest = ingest_run(config, run)
    assert manifest["metrics"]["revised_accuracy"] == 0.3
    assert "config.seed" not in manifest["metrics"]
    assert search_runs(config, "demo")[0]["run_id"] == "demo_run"
    assert get_run(config, "demo_run")["metrics"]["n"] == 10


def test_ingest_rejects_non_log_directory(tmp_path: Path) -> None:
    config, _ = config_for(tmp_path)
    outside = tmp_path / "project" / "hvs5" / "checkpoints" / "run"
    outside.mkdir(parents=True)
    with pytest.raises(ValueError, match="ingest root"):
        ingest_run(config, outside)
