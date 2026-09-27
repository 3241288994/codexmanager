---
name: codexmanager-setup
description: Guide safe CodexManager and LabContext setup so ChatGPT can inspect authorized local or server project paths. Use when the user asks to install, connect, verify, diagnose, or publish CodexManager integrations.
---

# CodexManager setup

1. Start from the user's outcome: paste an authorized path, inspect a bounded
   tree, find entry files, and explain the project. Then identify the boundary:
   - Web console: `http://127.0.0.1:48761/`.
   - Model tools: LabContext Router at `http://127.0.0.1:1460/mcp`.
   - Providers: loopback `/mcp` and a separate authenticated `/admin` surface.
   - Internal administration: `/api/rpc` through Web or `/rpc` on the service.
   - The public CodexManager service itself does not expose `/mcp` or a public
     OpenAI-compatible `/v1` gateway.
2. Prefer loopback binding plus SSH port forwarding for private deployments.
3. Never ask the user to publish databases, RPC tokens, account exports,
   `auth.json`, API keys, logs, or the data volume.
4. Keep local and server Provider permissions separate. Never authorize `/`, an
   entire home directory, credential stores, or unrelated project trees merely
   to simplify setup.
5. Before a remote/public deployment, require TLS, authentication, rate limits,
   least-privilege tools, redacted logs, and a separate administration origin.
6. Do not describe Secure MCP Tunnel as a public distribution mechanism. It is
   for private connectivity and developer testing; a public plugin needs a
   stable public HTTPS MCP endpoint.
7. For any write, delete, account switch, credential change, or public exposure,
   explain the target and obtain explicit confirmation before proceeding.

Success means ChatGPT can call `inspect_path` against each selected source and
return a bounded tree plus a real entry-file explanation; secrets remain outside
the repository, and administration routes are not model-visible.
