from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from labcontext_router import LabContextRouter, ProviderConfig, RouterConfig, RouterError  # noqa: E402


class FakeLabContextHandler(BaseHTTPRequestHandler):
    workspace_id = "shared"
    source_name = "unknown"
    calls: list[dict] = []

    def reply(self, value: dict | None, headers: dict[str, str] | None = None) -> None:
        body = json.dumps(value).encode() if value is not None else b""
        self.send_response(200)
        for key, item in (headers or {}).items():
            self.send_header(key, item)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:  # noqa: N802
        if self.path == "/admin/overview":
            self.reply({"default_workspace_id": self.workspace_id, "workspaces": []})
        else:
            self.send_error(404)

    def do_POST(self) -> None:  # noqa: N802
        length = int(self.headers.get("Content-Length", "0"))
        value = json.loads(self.rfile.read(length) or b"{}")
        self.__class__.calls.append(value)
        if self.path == "/admin/workspaces":
            self.reply({"ok": True, "workspace_id": "created"})
            return
        if self.path != "/mcp":
            self.send_error(404)
            return
        method = value.get("method")
        if method == "initialize":
            self.reply({"jsonrpc": "2.0", "id": value.get("id"), "result": {}}, {"Mcp-Session-Id": "fake-session"})
            return
        if method == "notifications/initialized":
            self.reply(None)
            return
        if method == "tools/list":
            tools = [
                {"name": "list_workspaces", "description": "List", "inputSchema": {"type": "object", "properties": {}}},
                {"name": "workspace_overview", "description": "Overview", "inputSchema": {"type": "object", "properties": {"workspace_id": {"type": "string"}}, "required": ["workspace_id"]}},
                {"name": "get_job", "description": "Job", "inputSchema": {"type": "object", "properties": {"job_id": {"type": "string"}}}},
            ]
            self.reply({"jsonrpc": "2.0", "id": value.get("id"), "result": {"tools": tools}})
            return
        params = value.get("params", {})
        tool = params.get("name")
        arguments = params.get("arguments", {})
        if tool == "list_workspaces":
            payload = {
                "default_workspace_id": self.workspace_id,
                "workspaces": [{"workspace_id": self.workspace_id, "name": f"{self.source_name} project"}],
            }
        elif tool == "get_job":
            payload = {"job_id": arguments.get("job_id"), "status": "completed"}
        else:
            payload = {"workspace_id": arguments.get("workspace_id"), "provider": self.source_name}
        result = {
            "content": [{"type": "text", "text": json.dumps(payload)}],
            "structuredContent": payload,
        }
        self.reply({"jsonrpc": "2.0", "id": value.get("id"), "result": result})

    def log_message(self, *_args: object) -> None:
        pass


def start_fake(name: str) -> tuple[ThreadingHTTPServer, str]:
    handler = type(f"{name.title()}Handler", (FakeLabContextHandler,), {
        "source_name": name,
        "calls": [],
    })
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server, f"http://127.0.0.1:{server.server_port}"


class RouterTest(unittest.TestCase):
    def setUp(self) -> None:
        self.local_server, local_url = start_fake("local")
        self.remote_server, remote_url = start_fake("server")
        providers = [
            ProviderConfig("local", "Local", f"{local_url}/mcp", f"{local_url}/admin"),
            ProviderConfig("server", "Server", f"{remote_url}/mcp", f"{remote_url}/admin"),
        ]
        self.router = LabContextRouter(RouterConfig(providers=providers))

    def tearDown(self) -> None:
        self.local_server.shutdown()
        self.remote_server.shutdown()
        self.local_server.server_close()
        self.remote_server.server_close()

    def test_lists_and_qualifies_workspaces_from_both_providers(self) -> None:
        result = self.router.call_tool("list_workspaces", {})
        payload = result["structuredContent"]
        self.assertEqual({item["workspace_ref"] for item in payload["workspaces"]}, {"local:shared", "server:shared"})
        self.assertEqual(payload["default_workspace_refs"], {"local": "local:shared", "server": "server:shared"})
        self.assertEqual({item["status"] for item in payload["sources"]}, {"ready"})

        local = self.router.call_tool("list_workspaces", {"source": "local"})["structuredContent"]
        self.assertEqual(local["default_workspace_id"], "shared")

    def test_tool_schema_accepts_workspace_ref_instead_of_workspace_id(self) -> None:
        tools, _ = self.router._tools(force=True)
        schema = next(item["inputSchema"] for item in tools if item["name"] == "workspace_overview")
        self.assertNotIn("required", schema)
        self.assertIn({"required": ["workspace_ref"]}, schema["allOf"][0]["anyOf"])

    def test_requires_a_qualified_reference_for_ambiguous_ids(self) -> None:
        with self.assertRaisesRegex(RouterError, "exists in multiple providers"):
            self.router.call_tool("workspace_overview", {"workspace_id": "shared"})
        result = self.router.call_tool("workspace_overview", {"workspace_ref": "local:shared"})
        self.assertEqual(result["structuredContent"]["provider"], "local")
        self.assertEqual(result["structuredContent"]["workspace_ref"], "local:shared")
        self.assertEqual(result["structuredContent"]["source"], "local")

    def test_job_ids_keep_their_provider(self) -> None:
        result = self.router.call_tool("get_job", {"job_id": "server:job-1"})
        self.assertEqual(result["structuredContent"]["job_id"], "server:job-1")
        self.assertEqual(result["structuredContent"]["source"], "server")

    def test_admin_bridge_maps_web_keys(self) -> None:
        result = self.router.admin_call("local", "upsertWorkspace", {
            "workspaceId": "one",
            "nestedValue": {"someKey": True},
        })
        self.assertEqual(result, {"ok": True, "workspaceId": "created"})
        calls = self.local_server.RequestHandlerClass.calls
        self.assertEqual(calls[-1]["workspace_id"], "one")
        self.assertEqual(calls[-1]["nested_value"], {"some_key": True})

    def test_provider_probe_reports_ready(self) -> None:
        statuses = self.router.provider_status(probe=True)
        self.assertEqual([item["status"] for item in statuses], ["ready", "ready"])
        self.assertEqual([item["adminStatus"] for item in statuses], ["ready", "ready"])


class LauncherTest(unittest.TestCase):
    def test_init_generates_decoupled_hybrid_config(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            config = Path(directory) / "config.toml"
            env_file = Path(directory) / "launcher.env"
            result = subprocess.run([
                sys.executable,
                str(SCRIPTS / "labcontext.py"),
                "--config", str(config),
                "--env-file", str(env_file),
                "init", "hybrid",
                "--ssh-host", "research-host",
                "--tunnel-profile", "labcontext",
            ], check=False, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            source = config.read_text(encoding="utf-8")
            self.assertIn('id = "local"', source)
            self.assertIn('id = "server"', source)
            self.assertIn("127.0.0.1:1456/mcp", source)
            self.assertIn("127.0.0.1:1455/mcp", source)
            environment = env_file.read_text(encoding="utf-8")
            self.assertIn("LABCONTEXT_ENABLE_SSH=1", environment)
            self.assertIn("LABCONTEXT_SSH_HOST=research-host", environment)


if __name__ == "__main__":
    unittest.main()
