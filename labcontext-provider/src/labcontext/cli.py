from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

import yaml

from .config import load_config
from .context import list_workspaces_data, workspace_overview_data
from .index import ingest_run
from .research_map import (
    apply_map_patch, codex_map_context, create_map_proposal, initialize_research_map, load_research_map,
    validate_research_map,
)
from .service import project_snapshot_data, run_server


def main() -> None:
    parser = argparse.ArgumentParser(prog="labctx")
    parser.add_argument("--config", type=Path, help="path to labcontext.toml")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("serve", help="start the loopback-only MCP HTTP server")
    commands.add_parser("snapshot", help="print project metadata")
    commands.add_parser("workspaces", help="list registered research workspaces")
    overview = commands.add_parser("overview", help="print a deterministic workspace overview")
    overview.add_argument("workspace_id", nargs="?")
    ingest = commands.add_parser("ingest-run", help="explicitly index one configured experiment directory")
    ingest.add_argument("run_dir", type=Path)
    map_command = commands.add_parser("map", help="inspect and safely update a workspace research map")
    map_commands = map_command.add_subparsers(dest="map_command", required=True)
    for name, help_text in (
        ("show", "print the canonical research map"),
        ("context", "print a compact Codex-facing YAML projection"),
        ("validate", "validate the canonical research map"),
        ("init", "create a deterministic starter map when none exists"),
    ):
        child = map_commands.add_parser(name, help=help_text)
        child.add_argument("workspace_id")
    apply_command = map_commands.add_parser("apply", help="validate and apply a domain map patch")
    apply_command.add_argument("workspace_id")
    apply_command.add_argument("patch_file", type=Path, help="JSON patch path, or - for stdin")
    apply_command.add_argument("--actor", default="codex-cli")
    propose_command = map_commands.add_parser("propose", help="submit a domain map patch for human review")
    propose_command.add_argument("workspace_id")
    propose_command.add_argument("patch_file", type=Path, help="JSON patch path, or - for stdin")
    propose_command.add_argument("--session-id")
    args = parser.parse_args()
    config = load_config(args.config)
    if args.command == "serve":
        run_server(config)
        return
    if args.command == "ingest-run":
        print(json.dumps(ingest_run(config, args.run_dir), indent=2, sort_keys=True))
        return
    if args.command == "workspaces":
        print(json.dumps(list_workspaces_data(config), indent=2, sort_keys=True))
        return
    if args.command == "map":
        if args.map_command == "show":
            print(json.dumps(load_research_map(config, args.workspace_id), ensure_ascii=False, indent=2))
            return
        if args.map_command == "context":
            print(yaml.safe_dump(codex_map_context(config, args.workspace_id), allow_unicode=True, sort_keys=False))
            return
        if args.map_command == "validate":
            workspace = config.workspaces.get(args.workspace_id)
            if workspace is None:
                from .context import resolve_workspace
                workspace = resolve_workspace(config, args.workspace_id)
            value = load_research_map(config, workspace.workspace_id)
            validate_research_map(value, workspace)
            print(json.dumps({"ok": True, "workspace_id": workspace.workspace_id,
                              "revision": value["revision"]}, indent=2))
            return
        if args.map_command == "init":
            print(json.dumps(initialize_research_map(config, args.workspace_id), ensure_ascii=False, indent=2))
            return
        if str(args.patch_file) == "-":
            patch = json.load(sys.stdin)
        else:
            patch = json.loads(args.patch_file.read_text(encoding="utf-8"))
        if args.map_command == "propose":
            print(json.dumps(create_map_proposal(
                config, args.workspace_id, patch, source_kind="codex_session",
                source_session_id=args.session_id,
            ), ensure_ascii=False, indent=2))
            return
        print(json.dumps(apply_map_patch(config, args.workspace_id, patch, actor=args.actor,
                                         proposal_source="cli" if args.actor.startswith("codex") else None),
                         ensure_ascii=False, indent=2))
        return
    if args.command == "overview":
        print(json.dumps(workspace_overview_data(config, args.workspace_id), indent=2, sort_keys=True))
        return
    print(json.dumps(project_snapshot_data(config), indent=2, sort_keys=True))
