# CodexManager Connector

This is a public, skills-only plugin. It is valid without an MCP connection and
guides safe CodexManager deployment and private client setup.

CodexManager itself exposes an OpenAI-compatible gateway, not an MCP endpoint.
Do not register its `/v1`, `/api/rpc`, `/rpc`, or LabContext admin routes as an
MCP server.

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
