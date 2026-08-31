# Changelog

## Unreleased

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
