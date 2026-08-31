#!/usr/bin/env python3
"""Add a user-specific registered MCP connection to a private plugin copy."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


ID_PATTERN = re.compile(r"^(?:plugin_)?asdk_app_[A-Za-z0-9_-]+$|^connector_[A-Za-z0-9_-]+$")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Create .app.json and wire it into a private plugin copy."
    )
    parser.add_argument("connection_id", help="Registered MCP technical ID")
    parser.add_argument("--name", default="codexmanager-mcp", help="Mapping name")
    args = parser.parse_args()

    if not ID_PATTERN.fullmatch(args.connection_id):
        parser.error(
            "connection_id must begin with plugin_asdk_app_, asdk_app_, or connector_"
        )
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]*", args.name):
        parser.error("--name must use lowercase letters, numbers, and hyphens")

    plugin_root = Path(__file__).resolve().parents[1]
    if any((parent / ".git").exists() for parent in plugin_root.parents):
        parser.error(
            "copy this plugin outside every Git worktree before configuring a "
            "user-specific connection"
        )
    manifest_path = plugin_root / ".codex-plugin" / "plugin.json"
    app_path = plugin_root / ".app.json"

    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["apps"] = "./.app.json"
    app_config = {"apps": {args.name: {"id": args.connection_id}}}

    app_path.write_text(json.dumps(app_config, indent=2) + "\n", encoding="utf-8")
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"Configured {args.connection_id} in {app_path}")
    print("Revalidate the plugin and restart ChatGPT/Codex before testing.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
