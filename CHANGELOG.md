# Changelog

## Unreleased

- Made LabContext recovery self-identifying: one-click repair can safely remove its own stale server-side SSH session and verified local orphan components, while refusing to claim unmarked or third-party processes.
- Upgraded LabContext Provider to 0.9.0 with one shared, bounded reader for direct paths, workspace files and evidence search, adding common source/config/log formats plus DOCX, XLSX, PPTX and Jupyter Notebook text extraction.
- Added the independently installable LabContext Provider 0.8.0 source package, with bounded HTML extraction, page-aware text PDF reading/search, explicit scanned-PDF OCR signaling, and local/server-independent deployment.
- Refined the research workspace console with visibility-aware polling, clearer server/local connection states, guided workspace setup, and a refreshed responsive interface.
- Added desktop-only local research workspaces with native folder selection, isolated server/local LabContext state, loopback-only administration, and documented local Secure MCP Tunnel topology.
- Added retained daily usage analytics, official-pricing reference synchronization, JSON/CSV export, and a refreshed analytics dashboard while keeping the new RPC surface administrator-only in multi-user mode.
- Prepared the repository for a clean public GitHub import.
- Removed bundled personal contact, payment, sponsor, referral, and external author-content defaults.
- Added secure LabContext Docker override guidance and support for Docker's local host gateway.
- Added the missing desktop Tauri bindings for session catalog and provider-index repair.
- Restored protected account-key, member dashboard, and request-log RPC dispatch, with explicit member self-data boundaries in accounts mode.
- Made the secure Compose profiles require a Docker-secret-backed Web access password and added first-start password bootstrapping.
- Made local upstream mock tests bypass ambient proxy settings so verification is reproducible across developer networks.
- Replaced the platform-specific source packer with a deterministic Python implementation.
- Added public-release preflight checks for runtime secrets, user-specific plugin connections, legacy image references, and personal assets.
- Excluded generated Tauri schemas and Python bytecode from source archives, and tightened the Docker build-context denylist for local credentials and generated artifacts.
- Made desktop updates opt in through `CODEXMANAGER_UPDATE_REPO` instead of inheriting an upstream repository, and added desktop-shell tests to CI.
- Removed unimplemented historical plugin-market, account-import/export, and warmup commands from the public Tauri invoke registry.

Historical release notes from the upstream project are intentionally not presented as releases of this repository.
