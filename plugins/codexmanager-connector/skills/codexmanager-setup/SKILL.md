---
name: codexmanager-setup
description: Guide a user through private CodexManager deployment checks, Codex CLI gateway configuration, and optional registration of a separate MCP server. Use when the user asks to deploy, connect, diagnose, or publish CodexManager integrations.
---

# CodexManager setup

1. Identify which boundary the user means:
   - Web console: `http://127.0.0.1:48761/`.
   - OpenAI-compatible gateway: `http://127.0.0.1:48761/v1` in all-in-one mode.
   - Internal administration: `/api/rpc` through Web or `/rpc` on the service.
   - MCP: a separate server; CodexManager does not currently expose `/mcp`.
2. Prefer loopback binding plus SSH port forwarding for private deployments.
3. Never ask the user to publish databases, RPC tokens, account exports,
   `auth.json`, API keys, logs, or the data volume.
4. When configuring Codex CLI, use a CodexManager platform key, not an OpenAI
   login access token or refresh token.
5. Before a remote/public deployment, require TLS, authentication, rate limits,
   least-privilege tools, redacted logs, and a separate administration origin.
6. Do not describe Secure MCP Tunnel as a public distribution mechanism. It is
   for private connectivity and developer testing; a public plugin needs a
   stable public HTTPS MCP endpoint.
7. For any write, delete, account switch, credential change, or public exposure,
   explain the target and obtain explicit confirmation before proceeding.

Success means the selected client can reach the intended boundary, secrets
remain outside the repository, and administration routes are not model-visible.
