use rusqlite::{backup::Backup, params, Connection, OpenFlags};
use serde::Serialize;
use std::{fs, path::Path, time::Duration};

const STATE_DB_FILE: &str = "state_5.sqlite";
const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 500;
const REPAIR_BACKUP_DIR: &str = "session-repair-backups";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionCatalogResult {
    pub codex_home: String,
    pub items: Vec<SessionCatalogItem>,
    pub total: i64,
    pub diagnostics: SessionCatalogDiagnostics,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionCatalogDiagnostics {
    pub state_db_available: bool,
    pub checked_count: i64,
    pub provider_mismatch_count: i64,
    pub missing_rollout_count: i64,
    pub missing_cwd_count: i64,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionCatalogItem {
    pub id: String,
    pub title: String,
    pub cwd: Option<String>,
    pub rollout_path: Option<String>,
    pub model_provider: Option<String>,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub updated_at_ms: i64,
    pub archived: bool,
    pub rollout_exists: bool,
    pub cwd_exists: bool,
    pub visibility: &'static str,
    pub resume_command: String,
}

struct RawSessionRow {
    id: String,
    title: Option<String>,
    first_user_message: Option<String>,
    cwd: Option<String>,
    rollout_path: Option<String>,
    model_provider: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<String>,
    updated_at_ms: Option<i64>,
    updated_at: Option<i64>,
    archived: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionIndexRepairResult {
    pub session_id: String,
    pub previous_provider: Option<String>,
    pub target_provider: &'static str,
    pub updated_rows: usize,
    pub backup_path: String,
    pub ledger_path: String,
    pub message: String,
}

pub(crate) fn list_sessions(
    codex_home: Option<&str>,
    query: Option<&str>,
    include_archived: bool,
    limit: Option<i64>,
) -> Result<SessionCatalogResult, String> {
    let profile_dir = crate::codex_profile::resolve_profile_dir(codex_home)?;
    let db_path = profile_dir.join(STATE_DB_FILE);
    if !db_path.exists() {
        return Ok(SessionCatalogResult {
            codex_home: profile_dir.to_string_lossy().to_string(),
            items: Vec::new(),
            total: 0,
            diagnostics: SessionCatalogDiagnostics {
                state_db_available: false,
                checked_count: 0,
                provider_mismatch_count: 0,
                missing_rollout_count: 0,
                missing_cwd_count: 0,
                message: format!("Codex state database not found: {}", db_path.display()),
            },
        });
    }

    let conn = open_read_only(&db_path)?;
    let columns = read_thread_columns(&conn)?;
    for required in ["id"] {
        if !columns.iter().any(|column| column == required) {
            return Err(format!(
                "Codex threads table is missing required column: {required}"
            ));
        }
    }

    let clean_query = query.map(str::trim).filter(|value| !value.is_empty());
    let pattern = clean_query.map(|value| format!("%{}%", escape_like(value)));
    let column = |name: &str| {
        if columns.iter().any(|candidate| candidate == name) {
            name.to_string()
        } else {
            "NULL".to_string()
        }
    };
    let archived_expr = column("archived");
    let archived_clause = if include_archived {
        "1 = 1".to_string()
    } else {
        format!("COALESCE({archived_expr}, 0) = 0")
    };
    let title_expr = column("title");
    let first_message_expr = column("first_user_message");
    let cwd_expr = column("cwd");
    let search_clause = if pattern.is_some() {
        format!("AND (id LIKE ?1 ESCAPE '\\' OR COALESCE({title_expr}, '') LIKE ?1 ESCAPE '\\' OR COALESCE({first_message_expr}, '') LIKE ?1 ESCAPE '\\' OR COALESCE({cwd_expr}, '') LIKE ?1 ESCAPE '\\')")
    } else {
        String::new()
    };
    let count_sql = format!("SELECT COUNT(*) FROM threads WHERE {archived_clause} {search_clause}");
    let total: i64 = if let Some(pattern) = pattern.as_deref() {
        conn.query_row(&count_sql, params![pattern], |row| row.get(0))
    } else {
        conn.query_row(&count_sql, [], |row| row.get(0))
    }
    .map_err(|err| format!("count Codex sessions failed: {err}"))?;

    let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let limit_param = if pattern.is_some() { "?2" } else { "?1" };
    let updated_at_ms_expr = column("updated_at_ms");
    let updated_at_expr = column("updated_at");
    let select_sql = format!(
        "SELECT id, {title_expr}, {first_message_expr}, {cwd_expr}, {}, {}, {}, {}, {updated_at_ms_expr}, {updated_at_expr}, {archived_expr} FROM threads WHERE {archived_clause} {search_clause} ORDER BY COALESCE({updated_at_ms_expr}, {updated_at_expr} * 1000, 0) DESC LIMIT {limit_param}",
        column("rollout_path"),
        column("model_provider"),
        column("model"),
        column("reasoning_effort"),
    );
    let mut statement = conn
        .prepare(&select_sql)
        .map_err(|err| format!("prepare Codex session query failed: {err}"))?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(RawSessionRow {
            id: row.get(0)?,
            title: row.get(1)?,
            first_user_message: row.get(2)?,
            cwd: row.get(3)?,
            rollout_path: row.get(4)?,
            model_provider: row.get(5)?,
            model: row.get(6)?,
            reasoning_effort: row.get(7)?,
            updated_at_ms: row.get(8)?,
            updated_at: row.get(9)?,
            archived: row.get(10)?,
        })
    };
    let rows = if let Some(pattern) = pattern.as_deref() {
        statement.query_map(params![pattern, limit], map_row)
    } else {
        statement.query_map(params![limit], map_row)
    }
    .map_err(|err| format!("read Codex sessions failed: {err}"))?;

    let mut items = Vec::new();
    let mut provider_mismatch_count = 0_i64;
    let mut missing_rollout_count = 0_i64;
    let mut missing_cwd_count = 0_i64;
    for row in rows {
        let raw = row.map_err(|err| format!("decode Codex session failed: {err}"))?;
        let rollout_exists = raw
            .rollout_path
            .as_deref()
            .map(Path::new)
            .is_some_and(Path::is_file);
        let cwd_exists = raw.cwd.as_deref().map(Path::new).is_some_and(Path::is_dir);
        let provider_matches = raw
            .model_provider
            .as_deref()
            .is_none_or(|provider| provider == "openai");
        if !provider_matches {
            provider_mismatch_count += 1;
        }
        if !rollout_exists {
            missing_rollout_count += 1;
        }
        if raw.cwd.is_some() && !cwd_exists {
            missing_cwd_count += 1;
        }
        let visibility = if !rollout_exists {
            "missing_rollout"
        } else if !provider_matches {
            "provider_mismatch"
        } else if raw.cwd.is_some() && !cwd_exists {
            "missing_workspace"
        } else {
            "ready"
        };
        let title = non_empty(raw.title)
            .or_else(|| non_empty(raw.first_user_message))
            .map(|value| truncate_chars(&value, 240))
            .unwrap_or_else(|| raw.id.clone());
        items.push(SessionCatalogItem {
            resume_command: build_resume_command(raw.cwd.as_deref(), &raw.id),
            id: raw.id,
            title,
            cwd: raw.cwd,
            rollout_path: raw.rollout_path,
            model_provider: raw.model_provider,
            model: raw.model,
            reasoning_effort: raw.reasoning_effort,
            updated_at_ms: raw
                .updated_at_ms
                .or_else(|| raw.updated_at.map(|value| value * 1000))
                .unwrap_or(0),
            archived: raw.archived.unwrap_or(0) != 0,
            rollout_exists,
            cwd_exists,
            visibility,
        });
    }

    let checked_count = items.len() as i64;
    Ok(SessionCatalogResult {
        codex_home: profile_dir.to_string_lossy().to_string(),
        items,
        total,
        diagnostics: SessionCatalogDiagnostics {
            state_db_available: true,
            checked_count,
            provider_mismatch_count,
            missing_rollout_count,
            missing_cwd_count,
            message: "Read-only catalog built from state_5.sqlite; session bodies were not scanned"
                .to_string(),
        },
    })
}

pub(crate) fn repair_provider_index(
    codex_home: Option<&str>,
    session_id: &str,
    confirm_session_id: &str,
) -> Result<SessionIndexRepairResult, String> {
    let session_id = session_id.trim();
    if session_id.is_empty() || session_id != confirm_session_id.trim() {
        return Err("confirmation must exactly match sessionId".to_string());
    }
    let profile_dir = crate::codex_profile::resolve_profile_dir(codex_home)?;
    let db_path = profile_dir.join(STATE_DB_FILE);
    let mut conn = Connection::open(&db_path).map_err(|err| {
        format!(
            "open Codex state database failed ({}): {err}",
            db_path.display()
        )
    })?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|err| format!("configure Codex state database timeout failed: {err}"))?;
    let columns = read_thread_columns(&conn)?;
    if !columns.iter().any(|column| column == "model_provider") {
        return Err(
            "Codex threads table has no model_provider column; no repair was attempted".to_string(),
        );
    }
    let previous_provider = conn
        .query_row(
            "SELECT model_provider FROM threads WHERE id = ?1",
            params![session_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(|err| format!("read target Codex session failed: {err}"))?;
    if previous_provider
        .as_deref()
        .is_none_or(|provider| provider == "openai")
    {
        return Err("session provider index already uses openai; no repair was needed".to_string());
    }

    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let safe_id = session_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .take(24)
        .collect::<String>();
    let backup_dir = profile_dir
        .join(REPAIR_BACKUP_DIR)
        .join(format!("{timestamp}-{safe_id}"));
    fs::create_dir_all(&backup_dir)
        .map_err(|err| format!("create session repair backup directory failed: {err}"))?;
    harden_private_dir(&backup_dir)?;
    let backup_path = backup_dir.join(STATE_DB_FILE);
    let mut backup_conn = Connection::open(&backup_path)
        .map_err(|err| format!("create session repair database backup failed: {err}"))?;
    let backup = Backup::new(&conn, &mut backup_conn)
        .map_err(|err| format!("start session repair database backup failed: {err}"))?;
    backup
        .run_to_completion(64, Duration::from_millis(25), None)
        .map_err(|err| format!("write session repair database backup failed: {err}"))?;
    drop(backup);
    drop(backup_conn);
    harden_private_file(&backup_path)?;

    let ledger_path = backup_dir.join("repair.json");
    let mut ledger = serde_json::json!({
        "version": 1,
        "status": "prepared",
        "sessionId": session_id,
        "previousProvider": previous_provider,
        "targetProvider": "openai",
        "stateDatabase": db_path,
        "backup": backup_path,
        "createdAt": chrono::Utc::now().to_rfc3339(),
        "scope": "sqlite_index_only",
        "rolloutModified": false
    });
    write_json(&ledger_path, &ledger)?;

    let transaction = conn
        .transaction()
        .map_err(|err| format!("start session index repair transaction failed: {err}"))?;
    let updated_rows = transaction
        .execute(
            "UPDATE threads SET model_provider = 'openai' WHERE id = ?1 AND COALESCE(model_provider, '') <> 'openai'",
            params![session_id],
        )
        .map_err(|err| format!("update session provider index failed: {err}"))?;
    if updated_rows != 1 {
        return Err(format!(
            "expected to repair exactly one session row, updated {updated_rows}"
        ));
    }
    transaction
        .commit()
        .map_err(|err| format!("commit session index repair failed: {err}"))?;
    ledger["status"] = serde_json::Value::String("completed".to_string());
    ledger["completedAt"] = serde_json::Value::String(chrono::Utc::now().to_rfc3339());
    write_json(&ledger_path, &ledger)?;

    Ok(SessionIndexRepairResult {
        session_id: session_id.to_string(),
        previous_provider,
        target_provider: "openai",
        updated_rows,
        backup_path: backup_path.to_string_lossy().to_string(),
        ledger_path: ledger_path.to_string_lossy().to_string(),
        message:
            "Only the selected SQLite index row was changed; the rollout file was not modified"
                .to_string(),
    })
}

fn write_json(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let body = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("encode session repair ledger failed: {err}"))?;
    fs::write(path, body).map_err(|err| {
        format!(
            "write session repair ledger failed ({}): {err}",
            path.display()
        )
    })?;
    harden_private_file(path)
}

#[cfg(unix)]
fn harden_private_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|err| {
        format!(
            "set private file permissions failed ({}): {err}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn harden_private_file(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn harden_private_dir(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|err| {
        format!(
            "set private directory permissions failed ({}): {err}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn harden_private_dir(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn open_read_only(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|err| {
        format!(
            "open Codex state database failed ({}): {err}",
            path.display()
        )
    })
}

fn read_thread_columns(conn: &Connection) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare("PRAGMA table_info(threads)")
        .map_err(|err| format!("inspect Codex threads table failed: {err}"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("read Codex thread columns failed: {err}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("decode Codex thread columns failed: {err}"))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn build_resume_command(cwd: Option<&str>, session_id: &str) -> String {
    let resume = format!("codex resume {}", shell_quote(session_id));
    match cwd.map(str::trim).filter(|value| !value.is_empty()) {
        Some(cwd) => format!("cd {} && {resume}", shell_quote(cwd)),
        None => resume,
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn resume_command_quotes_workspace_and_session() {
        assert_eq!(
            build_resume_command(Some("/tmp/research work"), "abc-123"),
            "cd '/tmp/research work' && codex resume 'abc-123'"
        );
    }

    #[test]
    fn like_query_is_escaped() {
        assert_eq!(escape_like("a%b_c\\d"), "a\\%b\\_c\\\\d");
    }

    #[test]
    fn long_titles_are_bounded_without_breaking_unicode() {
        assert_eq!(truncate_chars("科研工作区", 3), "科研工…");
        assert_eq!(truncate_chars("HVS", 10), "HVS");
    }

    #[test]
    fn provider_repair_is_single_row_and_creates_backup_ledger() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let profile_dir = std::env::temp_dir().join(format!(
            "codexmanager-session-repair-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&profile_dir).expect("create temporary profile");
        let db_path = profile_dir.join(STATE_DB_FILE);
        let conn = Connection::open(&db_path).expect("create state db");
        conn.execute(
            "CREATE TABLE threads (id TEXT PRIMARY KEY, model_provider TEXT)",
            [],
        )
        .expect("create threads");
        conn.execute(
            "INSERT INTO threads (id, model_provider) VALUES ('session-a', 'legacy'), ('session-b', 'legacy')",
            [],
        )
        .expect("insert sessions");
        drop(conn);

        let result = repair_provider_index(profile_dir.to_str(), "session-a", "session-a")
            .expect("repair selected session");
        assert_eq!(result.updated_rows, 1);
        assert!(Path::new(&result.backup_path).is_file());
        assert!(Path::new(&result.ledger_path).is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let backup_dir = Path::new(&result.backup_path)
                .parent()
                .expect("backup directory");
            assert_eq!(
                fs::metadata(backup_dir)
                    .expect("backup directory metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(&result.backup_path)
                    .expect("backup metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(&result.ledger_path)
                    .expect("ledger metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        let conn = Connection::open(&db_path).expect("reopen state db");
        let first: String = conn
            .query_row(
                "SELECT model_provider FROM threads WHERE id='session-a'",
                [],
                |row| row.get(0),
            )
            .expect("read repaired provider");
        let second: String = conn
            .query_row(
                "SELECT model_provider FROM threads WHERE id='session-b'",
                [],
                |row| row.get(0),
            )
            .expect("read untouched provider");
        assert_eq!(first, "openai");
        assert_eq!(second, "legacy");
        drop(conn);
        fs::remove_dir_all(&profile_dir).expect("remove temporary profile");
    }
}
