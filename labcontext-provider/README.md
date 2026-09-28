# LabContext

LabContext is a loopback-only, read-only MCP evidence gateway for research
workspaces. It lets ChatGPT discover a configured workspace, inspect bounded
evidence and experiment records, and request an asynchronous Codex analysis
without exposing arbitrary shell or filesystem operations.

## Public MCP tools

The model-visible surface is intentionally fixed at eleven tools:

1. `list_workspaces()` — discover registered workspaces, aliases and capabilities.
2. `workspace_overview(workspace_id?)` — fast deterministic default overview.
3. `research_context(workspace_id?, research_id?, sections?)` — normalized
   objectives, claims, evidence, decisions and open questions.
4. `search_evidence(query, workspace_id?, scopes?, limit?)` — search configured
   assets and return stable evidence references.
5. `get_evidence(evidence_refs, workspace_id?, detail?)` — retrieve bounded
   excerpts with path, location and content hashes.
6. `inspect_file(path, workspace_id?, view?, ...)` — inspect a known safe project
   file by Markdown/JSON outline, bounded search, line range, JSON Pointer or a
   bounded read, even when the file is not search-indexed.
7. `inspect_path(path, ...)` — inspect an absolute supported file, or a
   bounded directory tree below `registry.allowed_roots` without registering a
   workspace.
8. `query_experiments(...)` — refresh and query normalized experiment summaries.
9. `compare_experiments(...)` — align metrics and calculate server-side deltas.
10. `request_analysis(...)` — start or reuse a read-only Codex analysis job.
11. `get_job(job_id, wait_seconds?)` — read progress, failure or completed output.

`workspace_id` is optional. An omitted value resolves to the configured default;
every result reports the resolved ID. Tool calls never mutate a process-global
current directory, so different ChatGPT conversations can discuss different
workspaces safely.

Normal prompts can be short:

```text
总结当前工作区概述。
比较 baseline 和 treatment 的 accuracy，并给出证据。
结合最新 Codex 会话深入分析当前未解决问题。
```

The last request may invoke `request_analysis`; the first two should normally use
deterministic tools only.

When a Codex response or research record already names an exact project file,
`inspect_file` avoids a second fuzzy search. It accepts workspace-relative paths
or absolute paths that resolve inside the selected registered workspace. Direct
inspection uses the same format registry and parsers as `inspect_path`, with 10 MB input files and the global
response budget; denied directories, credentials, unsupported binary files, directories,
globs and paths outside the workspace remain inaccessible. Large JSON artifacts
should be opened with `outline` and then a targeted `json_pointer` or `search`
view rather than transferred in full.

`workspace_overview` also returns `working_context`: at most three recent,
workspace-matched Codex CLI session digests from the last 14 days. Each digest
contains a bounded objective, current focus and latest visible update. It is
derived only from user/assistant messages, with obvious credentials redacted;
raw transcripts and model reasoning are never returned. The whole section is
labelled `unverified_session_digest`, so it can provide continuity but cannot be
used as experimental or research evidence. These limits can be tuned under
`[summarizer]` with `recent_session_limit` (hard-capped at 3),
`recent_session_max_age_days`, and `recent_session_digest_chars`.

`inspect_path` is the quick, stateless path mode: paste an exact absolute path to
read one supported file, or list a directory up to three
levels deep. HTML extraction ignores active and hidden document elements and
never fetches referenced resources. PDF extraction is page-aware: use
`view="pages"` with `start_page`/`end_page`, or `view="search"` with a query.
Scanned PDFs are reported as requiring OCR rather than returning fabricated
text. It does not create registry entries or `.labcontext` files. Workspace
tools remain the persistent project mode for research context, evidence,
experiments and Codex handoff. Direct paths still have to remain below
`registry.allowed_roots` and are subject to credential, denied-path, 10 MB input,
500-page PDF, 20-page-per-call and response-size limits.

## Shared file readers (Provider 0.9.0)

`inspect_path`, `inspect_file` and `search_evidence` use the shared registry in
`file_types.py` and the readers in `file_reader.py`. Registering a workspace does
not change the supported formats. Workspace file reads additionally issue stable
hash-checked evidence references. Both inspection tools support `auto`, `outline`,
`search`, `lines` and `full_bounded` where applicable; JSON supports `json_pointer`,
and PDF supports `pages`, `start_page` and `end_page`.

| Formats | Extracted content |
| --- | --- |
| Common code: JS/JSX/MJS/CJS, TS/TSX, Vue/Svelte, CSS/SCSS, Python, C/C++ headers, Rust, Go, Java/Kotlin, C#, Swift, Ruby, PHP, shell/PowerShell, SQL and more | Source text; never executed |
| Markdown/RST/LaTeX, JSON/JSONL, YAML/TOML, INI/CONF/XML, CSV/TSV, LOG, DIFF/PATCH, SVG | Text (SVG source, not image understanding) |
| README, LICENSE, Dockerfile/Containerfile, Makefile, common lock/build files and ignore files | Known text filenames, including Dockerfile variants |
| HTML/HTM | Extracted body/title/headings; no scripts or network requests |
| PDF | Extractable text and page locations; scanned pages report `ocr_required` |
| DOCX | Main-document paragraphs and table text; no visual layout, images or embedded objects |
| XLSX | Sheet names, cell references and stored values; formulas are shown with cached results, not evaluated; dates may be stored serial values |
| PPTX | Text in presentation slide order, with slide labels; no image/chart interpretation or speaker notes |
| IPYNB | Cell source and plain-text outputs; no code execution, attachments or binary outputs |

The exhaustive extension/filename list is `src/labcontext/file_types.py`. Legacy
DOC/XLS/PPT, macro-enabled Office files, images/audio/video, archives and arbitrary
binary files are not supported. Convert legacy Office files to DOCX/XLSX/PPTX first.
Text decoding supports UTF-8 (with or without BOM), BOM-marked UTF-16/32 and GB18030;
ANSI color sequences are removed from log text. Sensitive credential names and
paths remain denied even when their extension is otherwise supported.

Single-file inspection retains the 10 MB limit. Office containers are read without
unpacking to disk and are limited to 2,000 members, 32 MB total uncompressed bytes,
and 8 MB per parsed XML part. DTD/entity declarations and encrypted archives are
rejected. Office/notebook extraction is capped at one million characters and
reports truncation; responses are bounded in UTF-8 bytes.

Workspace search still follows the configured asset include/exclude rules, scans
at most 5,000 candidates, skips inputs above 2 MB, and searches at most 200,000
extracted characters per file. Coverage reports size/parse skips and extraction
truncation. A file can therefore be individually readable without being searchable
under the current asset/size policy. Document references cite extracted-text lines
and, for PDFs, page ranges. A supplemental Provider evidence-location table stores
page coordinates without replacing existing evidence records; no CodexManager
schema change is needed. Changed files return `stale_reference` without new content.

Update **both** local and server Providers to 0.9.0 for the same capabilities on
both sources, then refresh the ChatGPT Connector tool metadata. The Web binary
does not need rebuilding for this Provider-only update.

## Workspace and asset registry

Workspaces are declared in `labcontext.toml`. A workspace has an ID, display
name, aliases, root, adapters and typed assets. Absolute roots must remain under
`registry.allowed_roots`.

In CodexManager, adding a workspace requires only a display name and an absolute
server path. LabContext derives a stable workspace ID, detects the core asset
types, updates the central registry, and creates one project-owned file:
`.labcontext/context.yaml`. That file contains the human-readable project
overview and its provenance. A newly registered workspace automatically starts
a read-only Codex overview job that combines targeted project inspection with
the latest matching CLI session. The resulting goal, current stage, recent work,
open questions and evidence references are written atomically to the YAML. The
overview can then be edited manually or explicitly regenerated from the card.
Adapter rules and discovered file counts remain in the central registry/index
rather than being copied into every project. Removing a workspace removes only
its registry entry; it never deletes the project directory or context file.

```toml
[registry]
default_workspace = "research"
allowed_roots = ["/srv/research"]

[workspaces.research]
name = "Research Project"
root = "/srv/research/project"
aliases = ["project"]
adapters = ["generic", "generic_experiments", "research_dossier"]

[[workspaces.research.assets]]
id = "experiments"
kind = "experiment_run"
include = ["runs/*/summary.json"]
adapter = "generic_experiments"
authority = "observed_artifact"
index_content = "metrics"
```

Supported core asset kinds are `project_docs`, `source_code`, `configuration`,
`experiment_run`, and `research_state`. New project formats should be added as
adapters that normalize their data; public MCP tool names should remain stable.

## Research map

Each workspace can maintain a small semantic graph for its core idea, claims,
explored branches, current target, experiments, evidence, decisions and risks.
The graph is initialized deterministically from reviewed context, structured
research state and indexed experiments; this first pass does not invoke Codex.
For generic projects, `current_stage` and `open_questions` in
`.labcontext/context.yaml` seed the current target and exploration branches.

Project-owned state is deliberately split into three files:

- `.labcontext/research-map.json` contains versioned research semantics;
- `.labcontext/research-map.layout.json` contains only canvas positions;
- `.labcontext/research-map.events.jsonl` is an append-only change timeline.

CodexManager renders the graph with focus/full/timeline views, node editing,
connections and independent layout saving. "Review with latest Codex" first
queues a bounded review request into the newest exact-workspace CLI session. If
that session is unavailable, an isolated read-only analysis worker is used.
Codex produces a revision-bound domain patch and never edits the layout or
applies its own proposal; a human must accept or reject it.

The MCP surface remains at eleven tools. `workspace_overview` includes a compact
`focus_capsule`, and `research_context` accepts `map_focus` and
`active_branches` sections. This gives ChatGPT the live research direction
without exposing the full editable graph on every request.

Useful maintenance commands are:

```bash
labctx map show hvs
labctx map init hvs
labctx map validate hvs
labctx map context hvs
labctx map propose hvs patch.json --session-id <session-id>
```

## Evidence and authority

Search results are registered as `ev_...` references. Reading a reference returns
its workspace, relative path, line range, indexed/current SHA-256 and authority.
If the file changed, the result is `stale_reference` instead of silently citing
new content under an old reference.

The intended authority vocabulary is:

- `canonical_fact`: reviewed contract, decision or equivalent project state;
- `observed_artifact`: code, config, metric or other directly observed file;
- `unverified_session_digest`: recent Codex conversation progress and judgments;
- `codex_inference`: analysis produced by a Codex worker.

## Codex analysis

`request_analysis` uses the configured model and reasoning effort in an isolated
`codex exec` process. The worker receives bounded Git/experiment/context/session
evidence, can inspect only the registered workspace read-only, and is instructed
not to run experiments, modify files, access denied assets, or use the network.

The evidence bundle is supplied directly on standard input, so Codex does not
spend extra tool rounds reading a temporary evidence file. Codex JSONL output is
drained continuously and mapped to planning, workspace-verification, synthesis
and validation progress. Diagnostic stderr is written to a private temporary
file instead of an unread pipe, preventing large tool traces from deadlocking
the worker. Completed jobs report elapsed time, event count, workspace-read
count, stderr byte count and token usage without exposing raw diagnostics.

The request returns immediately with a `job_id`; call `get_job` with that exact
ID. A restart converts a lost in-memory job to `interrupted`, preventing permanent
`running` records. The worker model and reasoning effort can be changed from
CodexManager. Changes are validated and hot-reloaded, apply only to newly created
analysis jobs, and never swap the model of a running job. Model and reasoning
defaults are deployment settings rather than package-level assumptions.

## Server operation

Start from the reviewed example configuration and keep the real configuration
outside the Git checkout. Replace every `/CHANGE_ME` path and keep
`registry.allowed_roots` limited to directories the user explicitly approved:

```bash
install -d -m 0700 ~/.config/labcontext
install -m 0600 labcontext.example.toml ~/.config/labcontext/provider.toml
${EDITOR:-vi} ~/.config/labcontext/provider.toml
labctx --config ~/.config/labcontext/provider.toml workspaces
```

In a hybrid setup the local Provider normally uses port `1456`, while the
server Provider remains on `1455` and is reached through the SSH bridge.

```bash
cd /opt/labcontext-provider
uv sync --extra dev
uv run labctx workspaces
uv run labctx overview
uv run labctx serve
```

The service binds only to `127.0.0.1:1455/mcp`. The installed user service starts
automatically because user lingering is enabled:

```bash
systemctl --user status labcontext
systemctl --user restart labcontext
journalctl --user -u labcontext -n 100 --no-pager
```

The Mac still needs the SSH local forward and Secure MCP Tunnel client. Server
startup is independent of those bridges.

## Local verification

```bash
uv run pytest -q
uv run labctx workspaces
uv run labctx overview hvs
```

Audit metadata is appended to `~/.local/state/labcontext/audit.jsonl`. Tool
inputs, raw evidence content and raw Codex transcripts are not written there.

## CodexManager control plane

CodexManager uses a separate loopback-only management surface under `/admin` to
show human-readable workspace cards, tool policy, worker configuration, analysis
jobs, recent audit activity and component health. It can set the registry
default, register a validated workspace from only a name and absolute path, edit
its overview, refresh an index and test selected deterministic tools.

The bearer token is created at
`~/.local/state/labcontext/admin.token` with `0600` permissions. Override the
location with `LABCTX_ADMIN_TOKEN_FILE`. Tool policy is persisted separately in
`~/.local/state/labcontext/tool-policy.json`; configuration updates are validated,
written atomically and backed up before the running service hot-reloads them.
The management routes are intentionally absent from MCP/OpenAPI discovery and
cannot be invoked by the ChatGPT model.

## Current non-goals

No arbitrary shell, unrestricted filesystem reads, experiment execution, raw transcript
export, checkpoint contents, or dataset contents. The CodexManager page may edit
only the bounded workspace registry and public-tool policy through the separate
authenticated management surface; the model-visible MCP surface remains
read-only.
