from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import os
import tomllib


@dataclass(frozen=True)
class ProjectConfig:
    project_id: str
    root: Path
    allowed_roots: tuple[str, ...]
    denied_globs: tuple[str, ...]
    max_response_bytes: int


@dataclass(frozen=True)
class ServerConfig:
    config_path: Path
    host: str
    port: int
    path: str
    project: ProjectConfig
    database: Path
    manifest_dir: Path
    ingest_roots: tuple[str, ...]
    workspaces: dict[str, "WorkspaceConfig"]
    default_workspace_id: str
    registry_allowed_roots: tuple[Path, ...]
    summarizer_command: tuple[str, ...]
    summarizer_timeout_seconds: int
    summarizer_model: str
    summarizer_reasoning_effort: str
    summarizer_max_context_bytes: int
    summarizer_session_dir: Path
    summarizer_max_session_chars: int
    summarizer_max_session_messages: int
    summarizer_recent_session_limit: int
    summarizer_recent_session_max_age_days: int
    summarizer_recent_session_digest_chars: int


@dataclass(frozen=True)
class WorkspaceConfig:
    workspace_id: str
    name: str
    root: Path
    context_file: Path
    aliases: tuple[str, ...]
    adapters: tuple[str, ...]
    assets: tuple["AssetConfig", ...]


@dataclass(frozen=True)
class AssetConfig:
    asset_id: str
    kind: str
    include: tuple[str, ...]
    exclude: tuple[str, ...]
    adapter: str
    authority: str
    index_content: str
    allow_codex: bool


def default_config_path() -> Path:
    return Path(os.environ.get("LABCTX_CONFIG", "labcontext.toml"))


def load_config(path: Path | None = None) -> ServerConfig:
    config_path = path or default_config_path()
    with config_path.open("rb") as handle:
        raw = tomllib.load(handle)
    server = raw["server"]
    project = raw["project"]
    limits = raw["limits"]
    storage = raw["storage"]
    registry = raw.get("registry", {})
    raw_workspaces = raw.get("workspaces", {})
    summarizer = raw["summarizer"]
    root = Path(project["root"]).expanduser().resolve()
    if not root.is_dir():
        raise ValueError(f"configured project root does not exist: {root}")
    if server["host"] not in {"127.0.0.1", "::1", "localhost"}:
        raise ValueError("LabContext phase 1 must bind only to loopback")
    registry_allowed_roots = tuple(
        Path(item).expanduser().resolve()
        for item in registry.get("allowed_roots", [str(root)])
    )
    if not registry_allowed_roots or not all(path.is_dir() for path in registry_allowed_roots):
        raise ValueError("every registry allowed root must be an existing directory")
    workspaces: dict[str, WorkspaceConfig] = {}
    for workspace_id, workspace in raw_workspaces.items():
        configured_root = Path(workspace["root"]).expanduser()
        workspace_root = (configured_root if configured_root.is_absolute() else root / configured_root).resolve()
        if not workspace_root.is_dir() or not any(
            workspace_root == allowed or workspace_root.is_relative_to(allowed)
            for allowed in registry_allowed_roots
        ):
            raise ValueError(f"invalid workspace root: {workspace_id}")
        context_file = (workspace_root / workspace.get("context_file", ".labcontext/context.yaml")).resolve()
        if not context_file.is_relative_to(workspace_root):
            raise ValueError(f"invalid workspace context file: {workspace_id}")
        assets = tuple(
            AssetConfig(
                asset_id=str(asset.get("id", f"asset-{index + 1}")),
                kind=str(asset.get("kind", "project_docs")),
                include=tuple(str(item) for item in asset.get("include", ["**/*"])),
                exclude=tuple(str(item) for item in asset.get("exclude", [])),
                adapter=str(asset.get("adapter", "generic")),
                authority=str(asset.get("authority", "observed_artifact")),
                index_content=str(asset.get("index_content", "metadata")),
                allow_codex=bool(asset.get("allow_codex", True)),
            )
            for index, asset in enumerate(workspace.get("assets", []))
        )
        workspaces[workspace_id] = WorkspaceConfig(
            workspace_id=workspace_id,
            name=str(workspace.get("name", workspace_id)),
            root=workspace_root,
            context_file=context_file,
            aliases=tuple(str(item) for item in workspace.get("aliases", [])),
            adapters=tuple(str(item) for item in workspace.get("adapters", ["generic"])),
            assets=assets,
        )
    if not workspaces:
        workspaces[project["id"].lower()] = WorkspaceConfig(
            workspace_id=project["id"].lower(), name=project["id"], root=root,
            context_file=(root / ".labcontext/context.yaml").resolve(), aliases=(),
            adapters=("generic",), assets=(),
        )
    default_workspace_id = str(registry.get("default_workspace", next(iter(workspaces))))
    if default_workspace_id not in workspaces:
        raise ValueError(f"unknown default workspace: {default_workspace_id}")
    return ServerConfig(
        config_path=config_path.expanduser().resolve(),
        host=server["host"], port=int(server["port"]), path=server["path"],
        project=ProjectConfig(
            project_id=project["id"], root=root,
            allowed_roots=tuple(project["allowed_roots"]),
            denied_globs=tuple(project["denied_globs"]),
            max_response_bytes=int(limits["max_response_bytes"]),
        ),
        database=Path(storage["database"]).expanduser(),
        manifest_dir=Path(storage["manifest_dir"]).expanduser(),
        ingest_roots=tuple(storage["ingest_roots"]),
        workspaces=workspaces,
        default_workspace_id=default_workspace_id,
        registry_allowed_roots=registry_allowed_roots,
        summarizer_command=tuple(summarizer["command"]),
        summarizer_timeout_seconds=int(summarizer["timeout_seconds"]),
        summarizer_model=str(summarizer.get("model", "gpt-5.6-terra")),
        summarizer_reasoning_effort=str(summarizer.get("reasoning_effort", "medium")),
        summarizer_max_context_bytes=int(summarizer["max_context_bytes"]),
        summarizer_session_dir=Path(summarizer.get("session_dir", "~/.codex/sessions")).expanduser(),
        summarizer_max_session_chars=int(summarizer.get("max_session_chars", 16000)),
        summarizer_max_session_messages=int(summarizer.get("max_session_messages", 24)),
        summarizer_recent_session_limit=max(1, min(int(summarizer.get("recent_session_limit", 3)), 3)),
        summarizer_recent_session_max_age_days=max(1, int(summarizer.get("recent_session_max_age_days", 14))),
        summarizer_recent_session_digest_chars=max(300, min(
            int(summarizer.get("recent_session_digest_chars", 900)), 1600,
        )),
    )
