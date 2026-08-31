# Frontend and desktop shell

`apps/` contains the statically exported Next.js UI and the Tauri v2 desktop shell.

The maintained user-facing routes are:

- `/`: account identity, usage snapshots, official device login, and verified Codex profile switching.
- `/sessions`: local `state_5.sqlite` session catalog and one-row provider-index repair.
- `/labcontext`: LabContext workspace, tool-policy, job, audit, and research-map console.

## Commands

```bash
corepack pnpm@10.30.3 install --frozen-lockfile
corepack pnpm@10.30.3 run dev
corepack pnpm@10.30.3 run build:desktop
corepack pnpm@10.30.3 run test:runtime
corepack pnpm@10.30.3 run test:e2e
```

The browser build needs `codexmanager-web` for `/api/runtime` and `/api/rpc`; a plain static file server is suitable only for the mocked Playwright test harness.

Desktop update checks are off until the maintainer configures `CODEXMANAGER_UPDATE_REPO=owner/repository` for this fork's own GitHub Releases. Never restore an upstream repository as an implicit fallback.

When adding a backend capability, update the typed client in `src/lib/api/`, the Web command mapping, the Tauri command registration when desktop support is required, and regression coverage. See [AGENTS.md](AGENTS.md) and the root [TESTING.md](../TESTING.md).
