# CodexManager Connector — Paste a path, understand the project

This public, skills-only plugin guides users through a safe CodexManager and
LabContext deployment: paste an authorized local or server path, inspect a
bounded directory tree, find entry files, and understand the project in ChatGPT.

The public CodexManager service is not the model-facing MCP endpoint and does
not publish an OpenAI-compatible `/v1` gateway. ChatGPT connects to the separate
LabContext Router `/mcp`; never register `/api/rpc`, `/rpc`, `/admin`, or an SSH
forward as an MCP server.

The public template deliberately contains no Tunnel ID, runtime key, registered
connection ID, `.app.json`, `apps`, or `mcpServers` entry. Install and test it
from a local marketplace before sharing it, following the OpenAI plugin
documentation.

If a separate, tested MCP service is added later, copy this directory outside
every Git worktree before adding a user-specific registered connection. The
optional helper creates `.app.json` and updates that private copy's manifest:

```bash
python3 scripts/configure_registered_connection.py plugin_asdk_app_your_id
```

The helper refuses to run inside a Git worktree. It does not create an OpenAI
Tunnel, register a ChatGPT connection, or validate the MCP service; it only
writes local plugin metadata. See
[`docs/open-source/03-openai-plugin-and-tunnel.md`](../../docs/open-source/03-openai-plugin-and-tunnel.md)
for the required MCP and Tunnel workflow.
