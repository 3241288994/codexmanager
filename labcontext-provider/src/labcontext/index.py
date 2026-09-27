from __future__ import annotations

from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import sqlite3
import subprocess
from typing import Any

from .config import ServerConfig, WorkspaceConfig


MAX_METRICS = 96
EXPERIMENT_EXTRACTOR_VERSION = 2
RUN_ID_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$")
SKIP_METRIC_PREFIXES = ("config.", "vector_info.", "by_source.")


def _private_directory(path: Path) -> None:
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    path.chmod(0o700)


def _connect(config: ServerConfig) -> sqlite3.Connection:
    _private_directory(config.database.parent)
    config.database.touch(mode=0o600, exist_ok=True)
    config.database.chmod(0o600)
    connection = sqlite3.connect(config.database)
    connection.row_factory = sqlite3.Row
    connection.execute(
        """CREATE TABLE IF NOT EXISTS runs (
            run_id TEXT PRIMARY KEY, title TEXT NOT NULL, source_rel TEXT NOT NULL,
            summary_rel TEXT NOT NULL, summary_sha256 TEXT NOT NULL, git_commit TEXT,
            metrics_json TEXT NOT NULL, ingested_at TEXT NOT NULL
        )"""
    )
    return connection


def _git_commit(root: Path) -> str | None:
    result = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "--short", "HEAD"],
        capture_output=True, text=True, check=False, timeout=5,
    )
    return result.stdout.strip() if result.returncode == 0 else None


def _numeric_metrics(value: Any, prefix: str = "") -> dict[str, float | int]:
    metrics: dict[str, float | int] = {}
    if isinstance(value, bool):
        return metrics
    if isinstance(value, (int, float)):
        skipped = any(
            prefix.startswith(skip) or f".{skip}" in prefix
            for skip in SKIP_METRIC_PREFIXES
        )
        if prefix and not skipped:
            metrics[prefix] = value
        return metrics
    if isinstance(value, dict):
        for key, child in value.items():
            child_prefix = f"{prefix}.{key}" if prefix else str(key)
            metrics.update(_numeric_metrics(child, child_prefix))
    return metrics


def _approved_run_directory(config: ServerConfig, candidate: Path) -> Path:
    run_dir = candidate.expanduser().resolve()
    for root_rel in config.ingest_roots:
        if run_dir.parent == (config.project.root / root_rel).resolve() and run_dir.is_dir():
            return run_dir
    raise ValueError("run must be a direct directory below a configured ingest root")


def ingest_run(config: ServerConfig, candidate: Path) -> dict[str, Any]:
    """Explicitly index one summary.json; raw run content never enters the index."""
    run_dir = _approved_run_directory(config, candidate)
    run_id = run_dir.name
    if not RUN_ID_PATTERN.fullmatch(run_id):
        raise ValueError("run directory name contains unsupported characters")
    summary_file = (run_dir / "summary.json").resolve()
    if not summary_file.is_file() or summary_file.parent != run_dir:
        raise ValueError("run directory must contain a direct summary.json file")
    raw_bytes = summary_file.read_bytes()
    try:
        summary = json.loads(raw_bytes)
    except json.JSONDecodeError as error:
        raise ValueError(f"summary.json is not valid JSON: {error.msg}") from error
    if not isinstance(summary, dict):
        raise ValueError("summary.json must contain a JSON object")

    metrics = _numeric_metrics(summary)
    if len(metrics) > MAX_METRICS:
        metrics = dict(sorted(metrics.items())[:MAX_METRICS])
    title = summary.get("run_label") if isinstance(summary.get("run_label"), str) else run_id
    manifest = {
        "schema_version": 1, "run_id": run_id, "title": title[:200],
        "source_rel": str(run_dir.relative_to(config.project.root)),
        "summary_rel": str(summary_file.relative_to(config.project.root)),
        "summary_sha256": hashlib.sha256(raw_bytes).hexdigest(),
        "git_commit": _git_commit(config.project.root), "metrics": metrics,
        "ingested_at": datetime.now(timezone.utc).isoformat(), "content_indexed": False,
    }
    _private_directory(config.manifest_dir)
    manifest_path = config.manifest_dir / f"{run_id}.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    manifest_path.chmod(0o600)
    with _connect(config) as connection:
        connection.execute(
            """INSERT INTO runs(run_id,title,source_rel,summary_rel,summary_sha256,git_commit,metrics_json,ingested_at)
               VALUES(:run_id,:title,:source_rel,:summary_rel,:summary_sha256,:git_commit,:metrics_json,:ingested_at)
               ON CONFLICT(run_id) DO UPDATE SET title=excluded.title,source_rel=excluded.source_rel,
               summary_rel=excluded.summary_rel,summary_sha256=excluded.summary_sha256,
               git_commit=excluded.git_commit,metrics_json=excluded.metrics_json,ingested_at=excluded.ingested_at""",
            {**manifest, "metrics_json": json.dumps(metrics, sort_keys=True)},
        )
    return manifest


def _record(row: sqlite3.Row, full_metrics: bool) -> dict[str, Any]:
    metrics = json.loads(row["metrics_json"])
    if not full_metrics:
        metrics = dict(list(metrics.items())[:16])
    return {
        "run_id": row["run_id"], "title": row["title"], "git_commit": row["git_commit"],
        "metrics": metrics, "ingested_at": row["ingested_at"],
        "evidence": {"summary": row["summary_rel"], "raw_content_indexed": False},
    }


def search_runs(config: ServerConfig, query: str, limit: int = 10) -> list[dict[str, Any]]:
    query = query.strip()
    if not query or len(query) > 200:
        raise ValueError("query must contain 1 to 200 characters")
    limit = max(1, min(int(limit), 20))
    needle = f"%{query.lower()}%"
    with _connect(config) as connection:
        rows = connection.execute(
            """SELECT * FROM runs WHERE lower(run_id) LIKE ? OR lower(title) LIKE ? OR lower(metrics_json) LIKE ?
               ORDER BY ingested_at DESC LIMIT ?""", (needle, needle, needle, limit),
        ).fetchall()
    return [_record(row, full_metrics=False) for row in rows]


def get_run(config: ServerConfig, run_id: str) -> dict[str, Any]:
    if not RUN_ID_PATTERN.fullmatch(run_id):
        raise ValueError("invalid run_id")
    with _connect(config) as connection:
        row = connection.execute("SELECT * FROM runs WHERE run_id = ?", (run_id,)).fetchone()
    if row is None:
        raise ValueError(f"unknown run_id: {run_id}")
    return _record(row, full_metrics=True)


def _ensure_experiment_table(config: ServerConfig) -> None:
    with _connect(config) as connection:
        connection.execute("""CREATE TABLE IF NOT EXISTS experiment_records (
            workspace_id TEXT NOT NULL, experiment_id TEXT NOT NULL, title TEXT NOT NULL,
            source_rel TEXT NOT NULL, summary_rel TEXT NOT NULL, summary_sha256 TEXT NOT NULL,
            git_commit TEXT, metrics_json TEXT NOT NULL, config_json TEXT NOT NULL,
            source_modified_at TEXT NOT NULL, indexed_at TEXT NOT NULL,
            extractor_version INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY(workspace_id, experiment_id)
        )""")
        columns = {row["name"] for row in connection.execute("PRAGMA table_info(experiment_records)")}
        if "extractor_version" not in columns:
            connection.execute("ALTER TABLE experiment_records ADD COLUMN extractor_version INTEGER NOT NULL DEFAULT 1")


def _experiment_summary_files(config: ServerConfig, workspace: WorkspaceConfig) -> list[Path]:
    candidates: set[Path] = set()
    for asset in workspace.assets:
        if asset.kind != "experiment_run":
            continue
        for pattern in asset.include:
            for path in workspace.root.glob(pattern):
                summary = path / "summary.json" if path.is_dir() else path
                if summary.name == "summary.json" and summary.is_file():
                    candidates.add(summary.resolve())
    for root_rel in config.ingest_roots:
        ingest_root = (config.project.root / root_rel).resolve()
        if not ingest_root.is_dir():
            continue
        try:
            ingest_root.relative_to(workspace.root)
        except ValueError:
            continue
        candidates.update(path.resolve() for path in ingest_root.glob("*/summary.json") if path.is_file())
    return sorted(candidates, key=lambda path: path.stat().st_mtime_ns, reverse=True)[:2000]


def refresh_experiment_index(config: ServerConfig, workspace: WorkspaceConfig) -> dict[str, int]:
    """Incrementally normalize configured summary.json assets for one workspace."""
    _ensure_experiment_table(config)
    discovered = indexed = unchanged = invalid = 0
    for summary_file in _experiment_summary_files(config, workspace):
        discovered += 1
        experiment_id = summary_file.parent.name
        if not RUN_ID_PATTERN.fullmatch(experiment_id):
            invalid += 1
            continue
        try:
            raw = summary_file.read_bytes()
            summary = json.loads(raw)
        except (OSError, json.JSONDecodeError):
            invalid += 1
            continue
        if not isinstance(summary, dict):
            invalid += 1
            continue
        digest = hashlib.sha256(raw).hexdigest()
        with _connect(config) as connection:
            existing = connection.execute(
                "SELECT summary_sha256,extractor_version FROM experiment_records WHERE workspace_id=? AND experiment_id=?",
                (workspace.workspace_id, experiment_id),
            ).fetchone()
        if existing and existing["summary_sha256"] == digest and existing["extractor_version"] == EXPERIMENT_EXTRACTOR_VERSION:
            unchanged += 1
            continue
        metrics = _numeric_metrics(summary)
        if len(metrics) > MAX_METRICS:
            metrics = dict(sorted(metrics.items())[:MAX_METRICS])
        raw_config = summary.get("config")
        normalized_config = raw_config if isinstance(raw_config, dict) else {}
        if len(json.dumps(normalized_config, ensure_ascii=False)) > 4_000:
            normalized_config = {"status": "omitted", "reason": "config exceeded index budget"}
        title = summary.get("run_label") if isinstance(summary.get("run_label"), str) else experiment_id
        modified_at = datetime.fromtimestamp(summary_file.stat().st_mtime, timezone.utc).isoformat()
        with _connect(config) as connection:
            connection.execute(
                """INSERT INTO experiment_records
                   (workspace_id,experiment_id,title,source_rel,summary_rel,summary_sha256,git_commit,
                    metrics_json,config_json,source_modified_at,indexed_at,extractor_version)
                   VALUES(?,?,?,?,?,?,?,?,?,?,?,?)
                   ON CONFLICT(workspace_id,experiment_id) DO UPDATE SET
                    title=excluded.title,source_rel=excluded.source_rel,summary_rel=excluded.summary_rel,
                    summary_sha256=excluded.summary_sha256,git_commit=excluded.git_commit,
                    metrics_json=excluded.metrics_json,config_json=excluded.config_json,
                    source_modified_at=excluded.source_modified_at,indexed_at=excluded.indexed_at,
                    extractor_version=excluded.extractor_version""",
                (
                    workspace.workspace_id, experiment_id, title[:200],
                    summary_file.parent.relative_to(workspace.root).as_posix(),
                    summary_file.relative_to(workspace.root).as_posix(), digest,
                    _git_commit(workspace.root), json.dumps(metrics, sort_keys=True),
                    json.dumps(normalized_config, sort_keys=True), modified_at,
                    datetime.now(timezone.utc).isoformat(), EXPERIMENT_EXTRACTOR_VERSION,
                ),
            )
        indexed += 1
    return {"discovered": discovered, "indexed": indexed, "unchanged": unchanged, "invalid": invalid}


def _experiment_record(row: sqlite3.Row, detail: str, metric_names: list[str]) -> dict[str, Any]:
    metrics = json.loads(row["metrics_json"])
    if metric_names:
        metrics = {name: metrics[name] for name in metric_names if name in metrics}
    elif detail == "summary":
        metrics = dict(list(metrics.items())[:16])
    else:
        metrics = dict(list(metrics.items())[:32])
    record: dict[str, Any] = {
        "experiment_id": row["experiment_id"], "title": row["title"],
        "source_modified_at": row["source_modified_at"], "git_commit": row["git_commit"],
        "metrics": metrics,
        "evidence": {"summary_path": row["summary_rel"], "summary_sha256": row["summary_sha256"]},
    }
    if detail == "manifest":
        record["source_path"] = row["source_rel"]
        record["config"] = json.loads(row["config_json"])
        record["indexed_at"] = row["indexed_at"]
    return record


def query_experiments(
    config: ServerConfig, workspace: WorkspaceConfig, query: str = "",
    experiment_ids: list[str] | None = None, metrics: list[str] | None = None,
    filters: dict[str, Any] | None = None, detail: str = "summary", limit: int = 10,
) -> dict[str, Any]:
    if detail not in {"summary", "metrics", "manifest"}:
        raise ValueError("detail must be summary, metrics, or manifest")
    experiment_ids = experiment_ids or []
    metrics = metrics or []
    filters = filters or {}
    if len(experiment_ids) > 50 or len(metrics) > 50:
        raise ValueError("too many experiment IDs or metrics")
    limit = max(1, min(int(limit), 50))
    refresh = refresh_experiment_index(config, workspace)
    clauses = ["workspace_id=?"]
    params: list[Any] = [workspace.workspace_id]
    if experiment_ids:
        clauses.append("experiment_id IN (" + ",".join("?" for _ in experiment_ids) + ")")
        params.extend(experiment_ids)
    if query.strip():
        needle = f"%{query.strip().lower()}%"
        clauses.append("(lower(experiment_id) LIKE ? OR lower(title) LIKE ? OR lower(metrics_json) LIKE ?)")
        params.extend([needle, needle, needle])
    with _connect(config) as connection:
        rows = connection.execute(
            f"SELECT * FROM experiment_records WHERE {' AND '.join(clauses)} ORDER BY source_modified_at DESC LIMIT ?",
            (*params, limit),
        ).fetchall()
    records = [_experiment_record(row, detail, metrics) for row in rows]
    for key, expected in filters.items():
        if key.startswith("metric."):
            metric = key.removeprefix("metric.")
            records = [record for record in records if record["metrics"].get(metric) == expected]
    bounded: list[dict[str, Any]] = []
    remaining = max(1000, config.project.max_response_bytes - 1200)
    response_truncated = False
    for record in records:
        encoded_size = len(json.dumps(record, ensure_ascii=False).encode())
        if encoded_size > remaining:
            compact = {**record, "metrics": dict(list(record["metrics"].items())[:8])}
            if "config" in compact:
                compact["config"] = {"status": "omitted", "reason": "response budget"}
            encoded_size = len(json.dumps(compact, ensure_ascii=False).encode())
            if encoded_size > remaining:
                response_truncated = True
                break
            record = compact
            response_truncated = True
        bounded.append(record)
        remaining -= encoded_size
    if len(bounded) < len(records):
        response_truncated = True
    return {
        "resolved_workspace_id": workspace.workspace_id, "experiments": bounded,
        "result_count": len(bounded), "response_truncated": response_truncated,
        "index_refresh": refresh,
    }


def compare_experiments(
    config: ServerConfig, workspace: WorkspaceConfig, experiment_ids: list[str],
    metrics: list[str] | None = None, include_config_diff: bool = False,
) -> dict[str, Any]:
    if len(experiment_ids) < 2 or len(experiment_ids) > 8:
        raise ValueError("compare 2 to 8 experiment IDs")
    result = query_experiments(
        config, workspace, experiment_ids=experiment_ids, metrics=metrics or [],
        detail="manifest" if include_config_diff else "metrics", limit=8,
    )
    records_by_id = {record["experiment_id"]: record for record in result["experiments"]}
    ordered = [records_by_id[experiment_id] for experiment_id in experiment_ids if experiment_id in records_by_id]
    missing = [experiment_id for experiment_id in experiment_ids if experiment_id not in records_by_id]
    metric_names = metrics or sorted({name for record in ordered for name in record["metrics"]})[:24]
    matrix = {
        metric: {record["experiment_id"]: record["metrics"].get(metric) for record in ordered}
        for metric in metric_names
    }
    baseline = ordered[0] if ordered else None
    deltas: dict[str, dict[str, float | None]] = {}
    if baseline:
        for metric in metric_names:
            base_value = baseline["metrics"].get(metric)
            deltas[metric] = {}
            for record in ordered[1:]:
                value = record["metrics"].get(metric)
                deltas[metric][record["experiment_id"]] = (
                    float(value) - float(base_value)
                    if isinstance(value, (int, float)) and isinstance(base_value, (int, float)) else None
                )
    config_diff: dict[str, dict[str, Any]] = {}
    if include_config_diff and ordered:
        keys = sorted({key for record in ordered for key in record.get("config", {})})[:30]
        config_diff = {
            key: {record["experiment_id"]: record.get("config", {}).get(key) for record in ordered}
            for key in keys
            if len({json.dumps(record.get("config", {}).get(key), sort_keys=True) for record in ordered}) > 1
        }
    sample_metrics = [name for name in metric_names if name.lower() in {"n", "count", "samples", "sample_count", "total"}]
    warnings = []
    if missing:
        warnings.append("Some requested experiments were not found")
    if sample_metrics and any(len({value for value in matrix[name].values() if value is not None}) > 1 for name in sample_metrics):
        warnings.append("Sample-count metrics differ; direct metric comparisons may not be equivalent")
    return {
        "resolved_workspace_id": workspace.workspace_id,
        "baseline_experiment_id": baseline["experiment_id"] if baseline else None,
        "experiment_ids": [record["experiment_id"] for record in ordered],
        "missing_experiment_ids": missing, "metric_matrix": matrix,
        "absolute_deltas_from_baseline": deltas, "config_diff": config_diff,
        "comparability_warnings": warnings,
        "evidence": {record["experiment_id"]: record["evidence"] for record in ordered},
    }
