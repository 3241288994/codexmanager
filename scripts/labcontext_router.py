#!/usr/bin/env python3
"""Small, dependency-free MCP router for optional LabContext providers.

The router exposes one Streamable HTTP MCP endpoint and dispatches calls to any
configured local or remote LabContext provider.  It also exposes a deliberately
small HTTP bridge used by the CodexManager Web UI for local administration.
"""

from __future__ import annotations

import argparse
import copy
import json
import os
import threading
import time
import uuid
from dataclasses import dataclass, field
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import ProxyHandler, Request, build_opener

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.9/3.10 compatibility
    tomllib = None  # type: ignore[assignment]


PROTOCOL_VERSION = "2025-06-18"
DEFAULT_LISTEN_ADDR = "127.0.0.1:1460"
DEFAULT_CONFIG_PATH = Path("~/.config/labcontext/config.toml").expanduser()
ADMIN_OPERATIONS: dict[str, tuple[str, str]] = {
    "overview": ("GET", "overview"),
    "setDefaultWorkspace": ("POST", "default-workspace"),
    "refreshWorkspace": ("POST", "refresh-workspace"),
    "testTool": ("POST", "test-tool"),
    "setToolPolicy": ("POST", "tool-policy"),
    "upsertWorkspace": ("POST", "workspaces"),
    "deleteWorkspace": ("POST", "delete-workspace"),
    "setWorkspaceOverview": ("POST", "workspace-overview"),
    "generateWorkspaceOverview": ("POST", "generate-workspace-overview"),
    "setWorkerConfig": ("POST", "worker-config"),
    "getResearchMap": ("GET", "research-map"),
    "initializeResearchMap": ("POST", "research-map/initialize"),
    "saveResearchMapLayout": ("POST", "research-map/layout"),
    "applyResearchMapPatch": ("POST", "research-map/patch"),
    "reviewResearchMap": ("POST", "research-map/review"),
    "researchMapProposalAction": ("POST", "research-map/proposal"),
}


class RouterError(RuntimeError):
    pass


def _as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def _snake_to_camel(key: str) -> str:
    head, *tail = key.split("_")
    return head + "".join(part[:1].upper() + part[1:] for part in tail)


def _camel_to_snake(key: str) -> str:
    output: list[str] = []
    for char in key:
        if char.isupper():
            output.extend(("_", char.lower()))
        else:
            output.append(char)
    return "".join(output)


def _map_keys(value: Any, mapper: Any) -> Any:
    if isinstance(value, dict):
        return {mapper(str(key)): _map_keys(item, mapper) for key, item in value.items()}
    if isinstance(value, list):
        return [_map_keys(item, mapper) for item in value]
    return value


def _parse_sse_or_json(raw: bytes, content_type: str) -> dict[str, Any]:
    text = raw.decode("utf-8")
    if "text/event-stream" not in content_type:
        value = json.loads(text or "{}")
        if not isinstance(value, dict):
            raise RouterError("upstream MCP returned a non-object response")
        return value
    for line in text.splitlines():
        if line.startswith("data: "):
            value = json.loads(line[6:])
            if isinstance(value, dict):
                return value
    raise RouterError("upstream MCP returned an empty event stream")


def _read_secret(path: str | None) -> str | None:
    if not path:
        return None
    try:
        value = Path(path).expanduser().read_text(encoding="utf-8").strip()
    except OSError as exc:
        raise RouterError(f"cannot read secret file {path}: {exc}") from exc
    if not value:
        raise RouterError(f"secret file is empty: {path}")
    return value


def _simple_toml_load(raw: bytes) -> dict[str, Any]:
    """Parse the deliberately small TOML subset used by router configuration."""
    result: dict[str, Any] = {}
    current = result
    for number, original in enumerate(raw.decode("utf-8").splitlines(), 1):
        line = original.strip()
        if not line or line.startswith("#"):
            continue
        if line == "[[providers]]":
            providers = result.setdefault("providers", [])
            if not isinstance(providers, list):
                raise RouterError("providers must be an array")
            current = {}
            providers.append(current)
            continue
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1].strip()
            if not section or not section.replace("_", "").isalnum():
                raise RouterError(f"unsupported TOML section on line {number}")
            value = result.setdefault(section, {})
            if not isinstance(value, dict):
                raise RouterError(f"duplicate TOML section on line {number}")
            current = value
            continue
        if "=" not in line:
            raise RouterError(f"invalid TOML assignment on line {number}")
        key, raw_value = (part.strip() for part in line.split("=", 1))
        try:
            if raw_value in {"true", "false"}:
                value: Any = raw_value == "true"
            elif raw_value.startswith(('"', "[")):
                value = json.loads(raw_value)
            elif "." in raw_value:
                value = float(raw_value)
            else:
                value = int(raw_value)
        except (json.JSONDecodeError, ValueError) as exc:
            raise RouterError(f"unsupported TOML value on line {number}") from exc
        current[key] = value
    return result


@dataclass
class ProviderConfig:
    id: str
    label: str
    mcp_url: str
    admin_url: str | None = None
    admin_token_file: str | None = None
    enabled: bool = True
    timeout_seconds: float = 20.0


@dataclass
class RouterConfig:
    listen_addr: str = DEFAULT_LISTEN_ADDR
    cors_origins: list[str] = field(default_factory=lambda: [
        "http://127.0.0.1:48761",
        "http://localhost:48761",
    ])
    providers: list[ProviderConfig] = field(default_factory=list)


def load_config(path: Path) -> RouterConfig:
    if not path.is_file():
        raise RouterError(
            f"router config not found: {path}; run `labcontext init <local|server|hybrid>` first"
        )
    with path.open("rb") as handle:
        raw_bytes = handle.read()
    raw = tomllib.loads(raw_bytes.decode("utf-8")) if tomllib else _simple_toml_load(raw_bytes)
    router = _as_dict(raw.get("router"))
    config = RouterConfig(
        listen_addr=str(router.get("listen_addr") or DEFAULT_LISTEN_ADDR),
        cors_origins=[str(item) for item in router.get("cors_origins", [])]
        or RouterConfig().cors_origins,
    )
    seen: set[str] = set()
    for item in raw.get("providers", []):
        source = _as_dict(item)
        provider_id = str(source.get("id") or "").strip()
        if not provider_id or ":" in provider_id:
            raise RouterError("provider id must be non-empty and cannot contain ':'")
        if provider_id in seen:
            raise RouterError(f"duplicate provider id: {provider_id}")
        seen.add(provider_id)
        mcp_url = str(source.get("mcp_url") or "").rstrip("/")
        if not mcp_url:
            raise RouterError(f"provider {provider_id} is missing mcp_url")
        config.providers.append(ProviderConfig(
            id=provider_id,
            label=str(source.get("label") or provider_id),
            mcp_url=mcp_url,
            admin_url=str(source.get("admin_url") or "").rstrip("/") or None,
            admin_token_file=str(source.get("admin_token_file") or "") or None,
            enabled=bool(source.get("enabled", True)),
            timeout_seconds=float(source.get("timeout_seconds", 20)),
        ))
    if not any(provider.enabled for provider in config.providers):
        raise RouterError("at least one provider must be enabled")
    return config


class UpstreamMcp:
    def __init__(self, config: ProviderConfig):
        self.config = config
        self._session_id: str | None = None
        self._lock = threading.RLock()
        self._opener = build_opener(ProxyHandler({}))

    def _request(self, payload: dict[str, Any], session: str | None = None) -> tuple[dict[str, Any], Any]:
        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream",
        }
        if session:
            headers["Mcp-Session-Id"] = session
        request = Request(
            self.config.mcp_url,
            data=json.dumps(payload, separators=(",", ":")).encode("utf-8"),
            headers=headers,
            method="POST",
        )
        try:
            response = self._opener.open(request, timeout=self.config.timeout_seconds)
            body = response.read()
            if not body:
                return dict(response.headers), None
            return dict(response.headers), _parse_sse_or_json(
                body,
                response.headers.get("Content-Type", ""),
            )
        except HTTPError as exc:
            detail = exc.read().decode("utf-8", errors="replace")[:500]
            raise RouterError(
                f"provider {self.config.id} returned HTTP {exc.code}: {detail or exc.reason}"
            ) from exc
        except (URLError, TimeoutError, OSError) as exc:
            raise RouterError(f"provider {self.config.id} is unavailable: {exc}") from exc

    def _initialize(self) -> None:
        headers, response = self._request({
            "jsonrpc": "2.0",
            "id": uuid.uuid4().hex,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "labcontext-router", "version": "0.1.0"},
            },
        })
        response_record = _as_dict(response)
        if response_record.get("error"):
            raise RouterError(f"provider {self.config.id} initialization failed: {response_record['error']}")
        self._session_id = headers.get("mcp-session-id") or headers.get("Mcp-Session-Id")
        if not self._session_id:
            raise RouterError(f"provider {self.config.id} did not return an MCP session id")
        self._request(
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            self._session_id,
        )

    def call(self, method: str, params: dict[str, Any]) -> dict[str, Any]:
        with self._lock:
            if not self._session_id:
                self._initialize()
            payload = {
                "jsonrpc": "2.0",
                "id": uuid.uuid4().hex,
                "method": method,
                "params": params,
            }
            try:
                _, response = self._request(payload, self._session_id)
            except RouterError as first_error:
                self._session_id = None
                try:
                    self._initialize()
                    _, response = self._request(payload, self._session_id)
                except RouterError:
                    raise first_error
            record = _as_dict(response)
            if record.get("error"):
                raise RouterError(f"provider {self.config.id}: {record['error']}")
            return _as_dict(record.get("result"))


class LabContextRouter:
    def __init__(self, config: RouterConfig):
        self.config = config
        self.providers = {
            provider.id: UpstreamMcp(provider)
            for provider in config.providers
            if provider.enabled
        }
        self._tool_cache: tuple[float, list[dict[str, Any]], dict[str, set[str]]] | None = None
        self._status_cache: tuple[float, list[dict[str, Any]]] | None = None
        self._cache_lock = threading.Lock()
        self._opener = build_opener(ProxyHandler({}))

    def provider_status(self, probe: bool = False) -> list[dict[str, Any]]:
        with self._cache_lock:
            if probe and self._status_cache and time.monotonic() - self._status_cache[0] < 5:
                return copy.deepcopy(self._status_cache[1])
        statuses: list[dict[str, Any]] = []
        for provider in self.providers.values():
            status: dict[str, Any] = {
                "id": provider.config.id,
                "label": provider.config.label,
                "mcpUrl": provider.config.mcp_url,
                "adminEnabled": bool(provider.config.admin_url),
                "status": "configured",
                "adminStatus": "configured" if provider.config.admin_url else "disabled",
            }
            if probe:
                try:
                    provider.call("tools/list", {})
                    status["status"] = "ready"
                except RouterError as exc:
                    status["status"] = "unavailable"
                    status["error"] = str(exc)
                if provider.config.admin_url:
                    try:
                        self.admin_call(provider.config.id, "overview", {})
                        status["adminStatus"] = "ready"
                    except RouterError as exc:
                        status["adminStatus"] = "unavailable"
                        status["adminError"] = str(exc)
            statuses.append(status)
        if probe:
            with self._cache_lock:
                self._status_cache = (time.monotonic(), copy.deepcopy(statuses))
        return statuses

    def _tools(self, force: bool = False) -> tuple[list[dict[str, Any]], dict[str, set[str]]]:
        with self._cache_lock:
            if not force and self._tool_cache and time.monotonic() - self._tool_cache[0] < 15:
                return self._tool_cache[1], self._tool_cache[2]
            merged: dict[str, dict[str, Any]] = {}
            availability: dict[str, set[str]] = {}
            errors: list[str] = []
            for provider_id, provider in self.providers.items():
                try:
                    tools = provider.call("tools/list", {}).get("tools", [])
                except RouterError as exc:
                    errors.append(str(exc))
                    continue
                for item in tools:
                    tool = _as_dict(item)
                    name = str(tool.get("name") or "")
                    if not name:
                        continue
                    availability.setdefault(name, set()).add(provider_id)
                    if name not in merged:
                        merged[name] = copy.deepcopy(tool)
            if not merged:
                raise RouterError("no provider tools are available: " + "; ".join(errors))
            for name, tool in merged.items():
                schema = _as_dict(tool.setdefault("inputSchema", {"type": "object"}))
                properties = _as_dict(schema.setdefault("properties", {}))
                source_ids = sorted(availability.get(name, set()))
                properties["source"] = {
                    "type": "string",
                    "enum": [*source_ids, "all"] if name == "list_workspaces" else source_ids,
                    "description": "Optional LabContext provider. Omit only when one source is configured or the workspace reference identifies it.",
                }
                if name not in {"list_workspaces", "get_job"}:
                    properties["workspace_ref"] = {
                        "type": "string",
                        "description": "Stable source-qualified workspace reference returned by list_workspaces, for example server:project-id.",
                    }
                    required = schema.get("required")
                    if isinstance(required, list) and "workspace_id" in required:
                        schema["required"] = [item for item in required if item != "workspace_id"]
                        if not schema["required"]:
                            schema.pop("required")
                        schema.setdefault("allOf", []).append({
                            "anyOf": [
                                {"required": ["workspace_id"]},
                                {"required": ["workspace_ref"]},
                            ],
                        })
                tool["description"] = (
                    str(tool.get("description") or "")
                    + " Routed by LabContext Router across optional providers."
                ).strip()
            result = list(merged.values())
            self._tool_cache = (time.monotonic(), result, availability)
            return result, availability

    @staticmethod
    def _workspace_payload(result: dict[str, Any]) -> dict[str, Any]:
        structured = result.get("structuredContent")
        if isinstance(structured, dict):
            return copy.deepcopy(structured)
        for content in result.get("content", []):
            item = _as_dict(content)
            if item.get("type") != "text":
                continue
            try:
                value = json.loads(str(item.get("text") or ""))
            except json.JSONDecodeError:
                continue
            if isinstance(value, dict):
                return value
        return {}

    @staticmethod
    def _tag_value(value: Any, provider: ProviderConfig) -> Any:
        if isinstance(value, dict):
            tagged = {key: LabContextRouter._tag_value(item, provider) for key, item in value.items()}
            workspace_id = tagged.get("workspace_id")
            if isinstance(workspace_id, str) and workspace_id:
                tagged.setdefault("workspace_ref", f"{provider.id}:{workspace_id}")
                tagged.setdefault("source", provider.id)
            job_id = tagged.get("job_id")
            if isinstance(job_id, str) and job_id and ":" not in job_id:
                tagged["job_id"] = f"{provider.id}:{job_id}"
                tagged.setdefault("source", provider.id)
            return tagged
        if isinstance(value, list):
            return [LabContextRouter._tag_value(item, provider) for item in value]
        return value

    def _tag_result(self, result: dict[str, Any], provider: ProviderConfig) -> dict[str, Any]:
        tagged = copy.deepcopy(result)
        structured = tagged.get("structuredContent")
        if isinstance(structured, dict):
            tagged["structuredContent"] = self._tag_value(structured, provider)
            tagged["structuredContent"].setdefault("source", provider.id)
        for item in tagged.get("content", []):
            record = _as_dict(item)
            if record.get("type") != "text":
                continue
            try:
                value = json.loads(str(record.get("text") or ""))
            except json.JSONDecodeError:
                continue
            record["text"] = json.dumps(
                self._tag_value(value, provider),
                ensure_ascii=False,
                separators=(",", ":"),
            )
        return tagged

    def _list_workspaces(self, source: str | None = None) -> dict[str, Any]:
        selected = self.providers
        if source and source != "all":
            provider = self.providers.get(source)
            if not provider:
                raise RouterError(f"unknown provider: {source}")
            selected = {source: provider}
        workspaces: list[dict[str, Any]] = []
        sources: list[dict[str, Any]] = []
        defaults: dict[str, str] = {}
        raw_defaults: dict[str, str] = {}
        for provider_id, provider in selected.items():
            try:
                upstream = provider.call("tools/call", {
                    "name": "list_workspaces",
                    "arguments": {},
                })
                payload = self._workspace_payload(upstream)
                default_id = payload.get("default_workspace_id")
                if isinstance(default_id, str) and default_id:
                    defaults[provider_id] = f"{provider_id}:{default_id}"
                    raw_defaults[provider_id] = default_id
                for workspace in payload.get("workspaces", []):
                    if isinstance(workspace, dict):
                        workspaces.append(self._tag_value(workspace, provider.config))
                sources.append({"id": provider_id, "label": provider.config.label, "status": "ready"})
            except RouterError as exc:
                sources.append({
                    "id": provider_id,
                    "label": provider.config.label,
                    "status": "unavailable",
                    "error": str(exc),
                })
        payload = {"sources": sources, "default_workspace_refs": defaults, "workspaces": workspaces}
        if len(selected) == 1 and raw_defaults:
            payload["default_workspace_id"] = next(iter(raw_defaults.values()))
        return {
            "content": [{"type": "text", "text": json.dumps(payload, ensure_ascii=False, separators=(",", ":"))}],
            "structuredContent": payload,
            "isError": not workspaces and all(item["status"] != "ready" for item in sources),
        }

    def _resolve_provider(self, tool: str, arguments: dict[str, Any]) -> tuple[UpstreamMcp, dict[str, Any]]:
        routed = copy.deepcopy(arguments)
        source = routed.pop("source", None)
        workspace_ref = routed.pop("workspace_ref", None)
        if isinstance(workspace_ref, str) and ":" in workspace_ref:
            ref_source, workspace_id = workspace_ref.split(":", 1)
            if source and source != ref_source:
                raise RouterError("source and workspace_ref refer to different providers")
            source = ref_source
            routed["workspace_id"] = workspace_id
        if tool == "get_job":
            job_id = routed.get("job_id")
            if isinstance(job_id, str) and ":" in job_id:
                job_source, raw_job_id = job_id.split(":", 1)
                if source and source != job_source:
                    raise RouterError("source and job_id refer to different providers")
                source = job_source
                routed["job_id"] = raw_job_id
        _, availability = self._tools()
        candidates = availability.get(tool, set())
        if source:
            if source == "all":
                raise RouterError("source=all is only valid for list_workspaces")
            provider = self.providers.get(str(source))
            if not provider or source not in candidates:
                raise RouterError(f"tool {tool} is not available from provider {source}")
            return provider, routed
        if len(candidates) == 1:
            provider_id = next(iter(candidates))
            return self.providers[provider_id], routed
        workspace_id = routed.get("workspace_id")
        if isinstance(workspace_id, str) and workspace_id:
            matches: list[str] = []
            listing = self._list_workspaces()
            for workspace in _as_dict(listing.get("structuredContent")).get("workspaces", []):
                record = _as_dict(workspace)
                if record.get("workspace_id") == workspace_id:
                    matches.append(str(record.get("source")))
            matches = sorted(set(matches))
            if len(matches) == 1:
                return self.providers[matches[0]], routed
            if len(matches) > 1:
                raise RouterError(
                    f"workspace_id {workspace_id!r} exists in multiple providers; use workspace_ref"
                )
        raise RouterError(
            f"multiple providers expose {tool}; pass source or a source-qualified workspace_ref"
        )

    def call_tool(self, name: str, arguments: dict[str, Any]) -> dict[str, Any]:
        if name == "list_workspaces":
            source = arguments.get("source")
            return self._list_workspaces(str(source) if source else None)
        provider, routed = self._resolve_provider(name, arguments)
        result = provider.call("tools/call", {"name": name, "arguments": routed})
        return self._tag_result(result, provider.config)

    @staticmethod
    def tool_error(message: str) -> dict[str, Any]:
        return {
            "content": [{"type": "text", "text": message}],
            "isError": True,
        }

    def handle_rpc(self, payload: dict[str, Any]) -> tuple[int, dict[str, Any] | None, str | None]:
        method = str(payload.get("method") or "")
        request_id = payload.get("id")
        if method == "notifications/initialized":
            return HTTPStatus.ACCEPTED, None, None
        if method == "initialize":
            result = {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {"tools": {"listChanged": True}},
                "serverInfo": {"name": "LabContext Router", "version": "0.1.0"},
                "instructions": (
                    "Unified LabContext router. Start with list_workspaces. Use workspace_ref from its "
                    "response whenever more than one provider is configured."
                ),
            }
            return HTTPStatus.OK, {"jsonrpc": "2.0", "id": request_id, "result": result}, uuid.uuid4().hex
        if method == "ping":
            return HTTPStatus.OK, {"jsonrpc": "2.0", "id": request_id, "result": {}}, None
        try:
            if method == "tools/list":
                tools, _ = self._tools(force=True)
                result = {"tools": tools}
            elif method == "tools/call":
                params = _as_dict(payload.get("params"))
                name = str(params.get("name") or "")
                if not name:
                    raise RouterError("tools/call requires a tool name")
                result = self.call_tool(name, _as_dict(params.get("arguments")))
            else:
                return HTTPStatus.OK, {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {"code": -32601, "message": f"method not found: {method}"},
                }, None
        except RouterError as exc:
            if method == "tools/call":
                result = self.tool_error(str(exc))
            else:
                return HTTPStatus.OK, {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {"code": -32000, "message": str(exc)},
                }, None
        return HTTPStatus.OK, {"jsonrpc": "2.0", "id": request_id, "result": result}, None

    def admin_call(self, provider_id: str, operation: str, params: dict[str, Any]) -> Any:
        provider = self.providers.get(provider_id)
        if not provider:
            raise RouterError(f"unknown provider: {provider_id}")
        config = provider.config
        if not config.admin_url:
            raise RouterError(f"provider {provider_id} does not expose an admin bridge")
        route = ADMIN_OPERATIONS.get(operation)
        if not route:
            raise RouterError(f"unsupported admin operation: {operation}")
        method, path = route
        query = ""
        body: bytes | None = None
        if method == "GET" and operation == "getResearchMap":
            workspace_id = str(params.get("workspaceId") or "")
            query = "?" + urlencode({"workspace_id": workspace_id})
        elif method == "POST":
            body = json.dumps(_map_keys(params, _camel_to_snake)).encode("utf-8")
        headers = {"Accept": "application/json"}
        token = _read_secret(config.admin_token_file)
        if token:
            headers["Authorization"] = f"Bearer {token}"
        if body is not None:
            headers["Content-Type"] = "application/json"
        request = Request(
            f"{config.admin_url}/{path}{query}",
            data=body,
            headers=headers,
            method=method,
        )
        try:
            response = self._opener.open(request, timeout=config.timeout_seconds)
            value = json.loads(response.read().decode("utf-8") or "{}")
            return _map_keys(value, _snake_to_camel)
        except HTTPError as exc:
            detail = exc.read().decode("utf-8", errors="replace")[:500]
            raise RouterError(f"provider {provider_id} admin returned HTTP {exc.code}: {detail}") from exc
        except (URLError, TimeoutError, OSError, json.JSONDecodeError) as exc:
            raise RouterError(f"provider {provider_id} admin call failed: {exc}") from exc


class RouterHandler(BaseHTTPRequestHandler):
    server_version = "LabContextRouter/0.1"

    @property
    def router(self) -> LabContextRouter:
        return self.server.router  # type: ignore[attr-defined]

    def _origin_allowed(self) -> str:
        origin = self.headers.get("Origin", "")
        if origin in self.router.config.cors_origins:
            return origin
        return ""

    def _require_allowed_origin(self) -> None:
        origin = self.headers.get("Origin")
        if origin and not self._origin_allowed():
            raise RouterError(f"origin is not allowed: {origin}")

    def _headers(self, status: int, content_type: str = "application/json; charset=utf-8") -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Cache-Control", "no-store")
        allowed_origin = self._origin_allowed()
        if allowed_origin:
            self.send_header("Access-Control-Allow-Origin", allowed_origin)
            self.send_header("Vary", "Origin")
        self.send_header("Access-Control-Allow-Private-Network", "true")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Mcp-Session-Id")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Expose-Headers", "Mcp-Session-Id")

    def _json(self, status: int, value: Any) -> None:
        body = json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
        self._headers(status)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _read_json(self) -> dict[str, Any]:
        length = int(self.headers.get("Content-Length", "0"))
        if length <= 0 or length > 2 * 1024 * 1024:
            raise RouterError("request body must be between 1 byte and 2 MiB")
        try:
            value = json.loads(self.rfile.read(length))
        except json.JSONDecodeError as exc:
            raise RouterError(f"invalid JSON request: {exc}") from exc
        if not isinstance(value, dict):
            raise RouterError("request body must be a JSON object")
        return value

    def do_OPTIONS(self) -> None:  # noqa: N802
        if self.headers.get("Origin") and not self._origin_allowed():
            self._json(HTTPStatus.FORBIDDEN, {"error": "origin_not_allowed"})
            return
        self._headers(HTTPStatus.NO_CONTENT)
        self.end_headers()

    def do_GET(self) -> None:  # noqa: N802
        try:
            self._require_allowed_origin()
            if self.path == "/healthz":
                self._json(HTTPStatus.OK, {"ok": True, "providers": self.router.provider_status(probe=True)})
                return
            if self.path == "/api/providers":
                self._json(HTTPStatus.OK, {"providers": self.router.provider_status(probe=True)})
                return
            self._json(HTTPStatus.NOT_FOUND, {"error": "not_found"})
        except RouterError as exc:
            self._json(HTTPStatus.FORBIDDEN, {"error": str(exc)})

    def do_DELETE(self) -> None:  # noqa: N802
        try:
            self._require_allowed_origin()
            if self.path != "/mcp":
                self._json(HTTPStatus.NOT_FOUND, {"error": "not_found"})
                return
            # Upstream sessions are pooled per provider rather than owned by a
            # downstream client, so terminating a downstream session is a no-op.
            self._headers(HTTPStatus.NO_CONTENT)
            self.end_headers()
        except RouterError as exc:
            self._json(HTTPStatus.FORBIDDEN, {"error": str(exc)})

    def do_POST(self) -> None:  # noqa: N802
        try:
            self._require_allowed_origin()
            payload = self._read_json()
            if self.path == "/mcp":
                status, response, session_id = self.router.handle_rpc(payload)
                if response is None:
                    self._headers(status)
                    self.end_headers()
                    return
                body = ("event: message\ndata: " + json.dumps(
                    response,
                    ensure_ascii=False,
                    separators=(",", ":"),
                ) + "\n\n").encode("utf-8")
                self._headers(status, "text/event-stream")
                if session_id:
                    self.send_header("Mcp-Session-Id", session_id)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
                return
            prefix = "/api/admin/"
            suffix = "/call"
            if self.path.startswith(prefix) and self.path.endswith(suffix):
                provider_id = self.path[len(prefix):-len(suffix)].strip("/")
                operation = str(payload.get("operation") or "")
                result = self.router.admin_call(provider_id, operation, _as_dict(payload.get("params")))
                self._json(HTTPStatus.OK, {"result": result})
                return
            self._json(HTTPStatus.NOT_FOUND, {"error": "not_found"})
        except RouterError as exc:
            self._json(HTTPStatus.BAD_REQUEST, {"error": str(exc)})
        except Exception as exc:  # pragma: no cover - defensive server boundary
            self._json(HTTPStatus.INTERNAL_SERVER_ERROR, {"error": f"router failure: {exc}"})

    def log_message(self, pattern: str, *args: Any) -> None:
        print(f"[labcontext-router] {self.address_string()} {pattern % args}")


class RouterServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, address: tuple[str, int], router: LabContextRouter):
        self.router = router
        super().__init__(address, RouterHandler)


def parse_listen_addr(value: str) -> tuple[str, int]:
    if value.count(":") != 1:
        raise RouterError("listen_addr must use host:port format")
    host, raw_port = value.rsplit(":", 1)
    if host not in {"127.0.0.1", "localhost"}:
        raise RouterError("labcontext-router must listen on an IPv4 loopback address")
    port = int(raw_port)
    if not 1 <= port <= 65535:
        raise RouterError("listen port is out of range")
    return host, port


def main() -> int:
    parser = argparse.ArgumentParser(description="Route one MCP connection across LabContext providers")
    parser.add_argument("--config", type=Path, default=DEFAULT_CONFIG_PATH)
    parser.add_argument("--check", action="store_true", help="validate configuration without starting")
    args = parser.parse_args()
    try:
        config = load_config(args.config.expanduser())
        address = parse_listen_addr(config.listen_addr)
        if args.check:
            print(json.dumps({
                "ok": True,
                "listenAddr": config.listen_addr,
                "providers": [provider.id for provider in config.providers if provider.enabled],
            }, ensure_ascii=False))
            return 0
        server = RouterServer(address, LabContextRouter(config))
        print(f"[labcontext-router] listening on http://{address[0]}:{address[1]}/mcp")
        try:
            server.serve_forever(poll_interval=0.5)
        except KeyboardInterrupt:
            pass
        finally:
            server.server_close()
        return 0
    except (RouterError, OSError, ValueError) as exc:
        print(f"labcontext-router: {exc}", file=os.sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
