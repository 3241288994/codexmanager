from pathlib import Path

import pytest

from labcontext.config import load_config


def write_config(path: Path, root: Path, host: str = "127.0.0.1") -> None:
    path.write_text(
        f'''[server]\nhost = "{host}"\nport = 1455\npath = "/mcp"\n\n[project]\nid = "test"\nroot = "{root}"\nallowed_roots = ["src"]\ndenied_globs = ["**/.env"]\n\n[limits]\nmax_response_bytes = 12\n\n[storage]\ndatabase = "{path.parent / 'state' / 'index.db'}"\nmanifest_dir = "{path.parent / 'state' / 'manifests'}"\ningest_roots = ["logs"]\n\n[summarizer]\ncommand = ["echo"]\ntimeout_seconds = 1\nmax_context_bytes = 100\n'''
    )


def test_loads_loopback_config(tmp_path: Path) -> None:
    config_path = tmp_path / "labcontext.toml"
    write_config(config_path, tmp_path)
    assert load_config(config_path).project.root == tmp_path.resolve()


def test_rejects_non_loopback_bind(tmp_path: Path) -> None:
    config_path = tmp_path / "labcontext.toml"
    write_config(config_path, tmp_path, host="0.0.0.0")
    with pytest.raises(ValueError, match="loopback"):
        load_config(config_path)
