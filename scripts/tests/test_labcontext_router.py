from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest import mock
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from labcontext import process_status, ssh_command  # noqa: E402
from labcontext_router import (  # noqa: E402
    LabContextRouter,
    ProviderConfig,
    RouterConfig,
    RouterError,
    RouterServer,
)


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
            self.reply({
                "jsonrpc": "2.0",
                "id": value.get("id"),
                "result": {
                    "protocolVersion": "2025-06-18",
                    "serverInfo": {"name": "LabContext", "version": "0.7.0"},
                },
            }, {"Mcp-Session-Id": "fake-session"})
            return
        if method == "notifications/initialized":
            self.reply(None)
            return
        if method == "tools/list":
            tools = [
                {"name": "list_workspaces", "description": "List", "inputSchema": {"type": "object", "properties": {}}},
                {"name": "workspace_overview", "description": "Overview", "inputSchema": {"type": "object", "properties": {"workspace_id": {"type": "string"}}, "required": ["workspace_id"]}},
                {"name": "get_job", "description": "Job", "inputSchema": {"type": "object", "properties": {"job_id": {"type": "string"}}}},
                {"name": "inspect_path", "description": "Direct path", "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}},
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
        elif tool == "inspect_path":
            payload = {"path": arguments.get("path"), "provider": self.source_name, "path_type": "file"}
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

    def test_direct_path_schema_requires_source_without_workspace_registration(self) -> None:
        tools, _ = self.router._tools(force=True)
        schema = next(item["inputSchema"] for item in tools if item["name"] == "inspect_path")
        self.assertEqual(set(schema["required"]), {"path", "source"})
        self.assertNotIn("workspace_ref", schema["properties"])
        result = self.router.call_tool("inspect_path", {
            "source": "server", "path": "/mnt/research/project/README.md",
        })
        self.assertEqual(result["structuredContent"]["provider"], "server")
        self.assertEqual(result["structuredContent"]["source"], "server")

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
        self.assertEqual([item["serverVersion"] for item in statuses], ["0.7.0", "0.7.0"])
        self.assertEqual([item["workspaceCount"] for item in statuses], [1, 1])

    def test_provider_probe_rejects_an_incompatible_identity(self) -> None:
        self.router.providers["local"].server_info = {"name": "Unexpected", "version": "9.0.0"}
        self.router.providers["local"]._session_id = "fake-session"
        statuses = self.router.provider_status(probe=True)
        self.assertEqual(statuses[0]["status"], "unavailable")
        self.assertIn("identity mismatch", statuses[0]["error"])

    def test_connection_status_requires_capability_checks_not_only_ports(self) -> None:
        with mock.patch.dict("os.environ", {"LABCONTEXT_ENABLE_TUNNEL": "0"}):
            status = self.router.connection_status()
        self.assertEqual(status["overall"], "ready")
        self.assertEqual(status["router"]["version"], "0.3.0")
        self.assertEqual(status["tunnel"]["status"], "disabled")
        self.assertEqual({item["serverName"] for item in status["providers"]}, {"LabContext"})

    def test_downstream_session_delete_is_accepted(self) -> None:
        server = RouterServer(("127.0.0.1", 0), self.router)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            from urllib.request import Request, urlopen

            request = Request(
                f"http://127.0.0.1:{server.server_port}/mcp",
                method="DELETE",
            )
            with urlopen(request, timeout=2) as response:
                self.assertEqual(response.status, 204)
            with urlopen(f"http://127.0.0.1:{server.server_port}/api/status", timeout=2) as response:
                status = json.loads(response.read())
                self.assertEqual(status["overall"], "ready")
                self.assertEqual(status["router"]["name"], "LabContext Router")
        finally:
            server.shutdown()
            server.server_close()


class LauncherTest(unittest.TestCase):
    def test_failed_optional_component_reports_retry_state(self) -> None:
        process = subprocess.Popen([sys.executable, "-c", "raise SystemExit(23)"])
        process.wait(timeout=2)
        status = process_status(
            "SSH bridge",
            process,
            {"SSH bridge": 0},
            {"SSH bridge": 10**12},
            {"SSH bridge": 2},
        )
        self.assertEqual(status["status"], "retrying")
        self.assertEqual(status["exitCode"], 23)
        self.assertEqual(status["restartAttempts"], 2)

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

    def test_ssh_command_preserves_identity_keepalive_and_reverse_proxy(self) -> None:
        command = ssh_command({
            "LABCONTEXT_SSH_HOST": "research-host",
            "LABCONTEXT_SSH_IDENTITY_FILE": "/private/key",
            "LABCONTEXT_SSH_CONFIG_FILE": "/dev/null",
            "LABCONTEXT_SERVER_MCP_LOCAL_PORT": "1455",
            "LABCONTEXT_SERVER_MCP_REMOTE_PORT": "1455",
            "LABCONTEXT_SERVER_WEB_LOCAL_PORT": "48761",
            "LABCONTEXT_SERVER_WEB_REMOTE_PORT": "48761",
            "LABCONTEXT_SERVER_PROXY_REMOTE_PORT": "17987",
            "LABCONTEXT_LOCAL_PROXY_PORT": "7897",
        })
        self.assertIn("/private/key", command)
        self.assertIn("ServerAliveInterval=30", command)
        self.assertIn("127.0.0.1:17987:127.0.0.1:7897", command)


if __name__ == "__main__":
    unittest.main()
