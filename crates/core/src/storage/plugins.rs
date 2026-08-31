use super::{now_ts, PluginInstall, PluginRunLog, PluginTask, Storage};
use rusqlite::{params, params_from_iter, types::Value, Result, Row};

impl Storage {
    /// 函数 `upsert_plugin_install`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - plugin: 参数 plugin
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn upsert_plugin_install(&self, plugin: &PluginInstall) -> Result<()> {
        self.conn.execute(
            "INSERT INTO plugin_installs (
                plugin_id, source_url, name, version, description, author, homepage_url, script_url,
                script_body, permissions_json, manifest_json, status, installed_at, updated_at,
                last_run_at, last_error
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(plugin_id) DO UPDATE SET
                source_url = excluded.source_url,
                name = excluded.name,
                version = excluded.version,
                description = excluded.description,
                author = excluded.author,
                homepage_url = excluded.homepage_url,
                script_url = excluded.script_url,
                script_body = excluded.script_body,
                permissions_json = excluded.permissions_json,
                manifest_json = excluded.manifest_json,
                status = excluded.status,
                updated_at = excluded.updated_at,
                last_run_at = excluded.last_run_at,
                last_error = excluded.last_error",
            params![
                &plugin.plugin_id,
                &plugin.source_url,
                &plugin.name,
                &plugin.version,
                &plugin.description,
                &plugin.author,
                &plugin.homepage_url,
                &plugin.script_url,
                &plugin.script_body,
                &plugin.permissions_json,
                &plugin.manifest_json,
                &plugin.status,
                plugin.installed_at,
                plugin.updated_at,
                plugin.last_run_at,
                &plugin.last_error,
            ],
        )?;
        Ok(())
    }

    /// 函数 `replace_plugin_install`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - plugin: 参数 plugin
    /// - tasks: 参数 tasks
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn replace_plugin_install(
        &self,
        plugin: &PluginInstall,
        tasks: &[PluginTask],
    ) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO plugin_installs (
                plugin_id, source_url, name, version, description, author, homepage_url, script_url,
                script_body, permissions_json, manifest_json, status, installed_at, updated_at,
                last_run_at, last_error
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(plugin_id) DO UPDATE SET
                source_url = excluded.source_url,
                name = excluded.name,
                version = excluded.version,
                description = excluded.description,
                author = excluded.author,
                homepage_url = excluded.homepage_url,
                script_url = excluded.script_url,
                script_body = excluded.script_body,
                permissions_json = excluded.permissions_json,
                manifest_json = excluded.manifest_json,
                status = excluded.status,
                updated_at = excluded.updated_at,
                last_run_at = excluded.last_run_at,
                last_error = excluded.last_error",
            params![
                &plugin.plugin_id,
                &plugin.source_url,
                &plugin.name,
                &plugin.version,
                &plugin.description,
                &plugin.author,
                &plugin.homepage_url,
                &plugin.script_url,
                &plugin.script_body,
                &plugin.permissions_json,
                &plugin.manifest_json,
                &plugin.status,
                plugin.installed_at,
                plugin.updated_at,
                plugin.last_run_at,
                &plugin.last_error,
            ],
        )?;
        tx.execute(
            "DELETE FROM plugin_tasks WHERE plugin_id = ?1",
            [&plugin.plugin_id],
        )?;
        for task in tasks {
            tx.execute(
                "INSERT INTO plugin_tasks (
                    id, plugin_id, name, description, entrypoint, schedule_kind, interval_seconds,
                    enabled, next_run_at, last_run_at, last_status, last_error, task_json, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    &task.id,
                    &task.plugin_id,
                    &task.name,
                    &task.description,
                    &task.entrypoint,
                    &task.schedule_kind,
                    &task.interval_seconds,
                    if task.enabled { 1_i64 } else { 0_i64 },
                    &task.next_run_at,
                    &task.last_run_at,
                    &task.last_status,
                    &task.last_error,
                    &task.task_json,
                    task.created_at,
                    task.updated_at,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// 函数 `list_plugin_installs`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn list_plugin_installs(&self) -> Result<Vec<PluginInstall>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                plugin_id, source_url, name, version, description, author, homepage_url, script_url,
                script_body, permissions_json, manifest_json, status, installed_at, updated_at,
                last_run_at, last_error
             FROM plugin_installs
             ORDER BY updated_at DESC, installed_at DESC, plugin_id ASC",
        )?;
        let mut rows = stmt.query([])?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(map_plugin_install_row(row)?);
        }
        Ok(items)
    }

    /// 函数 `find_plugin_install`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - plugin_id: 参数 plugin_id
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn find_plugin_install(&self, plugin_id: &str) -> Result<Option<PluginInstall>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                plugin_id, source_url, name, version, description, author, homepage_url, script_url,
                script_body, permissions_json, manifest_json, status, installed_at, updated_at,
                last_run_at, last_error
             FROM plugin_installs
             WHERE plugin_id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query([plugin_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(map_plugin_install_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// 函数 `update_plugin_install_status`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - plugin_id: 参数 plugin_id
    /// - status: 参数 status
    /// - last_error: 参数 last_error
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn update_plugin_install_status(
        &self,
        plugin_id: &str,
        status: &str,
        last_error: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE plugin_installs
             SET status = ?1, last_error = ?2, updated_at = ?3
             WHERE plugin_id = ?4",
            (status, last_error, now_ts(), plugin_id),
        )?;
        Ok(())
    }

    /// 函数 `update_plugin_install_last_run`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - plugin_id: 参数 plugin_id
    /// - last_run_at: 参数 last_run_at
    /// - last_error: 参数 last_error
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn update_plugin_install_last_run(
        &self,
        plugin_id: &str,
        last_run_at: i64,
        last_error: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE plugin_installs
             SET last_run_at = ?1, last_error = ?2, updated_at = ?3
             WHERE plugin_id = ?4",
            (last_run_at, last_error, now_ts(), plugin_id),
        )?;
        Ok(())
    }

    /// 函数 `delete_plugin_install`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - plugin_id: 参数 plugin_id
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn delete_plugin_install(&self, plugin_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM plugin_tasks WHERE plugin_id = ?1", [plugin_id])?;
        self.conn.execute(
            "DELETE FROM plugin_installs WHERE plugin_id = ?1",
            [plugin_id],
        )?;
        Ok(())
    }

    /// 函数 `list_plugin_tasks`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - plugin_id: 参数 plugin_id
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn list_plugin_tasks(&self, plugin_id: Option<&str>) -> Result<Vec<PluginTask>> {
        let sql = if plugin_id.is_some() {
            "SELECT id, plugin_id, name, description, entrypoint, schedule_kind, interval_seconds,
                enabled, next_run_at, last_run_at, last_status, last_error, task_json, created_at, updated_at
             FROM plugin_tasks
             WHERE plugin_id = ?1
             ORDER BY next_run_at ASC, created_at ASC"
        } else {
            "SELECT id, plugin_id, name, description, entrypoint, schedule_kind, interval_seconds,
                enabled, next_run_at, last_run_at, last_status, last_error, task_json, created_at, updated_at
             FROM plugin_tasks
             ORDER BY next_run_at ASC, created_at ASC"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = if let Some(plugin_id) = plugin_id {
            stmt.query([plugin_id])?
        } else {
            stmt.query([])?
        };
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(map_plugin_task_row(row)?);
        }
        Ok(items)
    }

    /// 函数 `find_plugin_task`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - task_id: 参数 task_id
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn find_plugin_task(&self, task_id: &str) -> Result<Option<PluginTask>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, plugin_id, name, description, entrypoint, schedule_kind, interval_seconds,
                enabled, next_run_at, last_run_at, last_status, last_error, task_json, created_at, updated_at
             FROM plugin_tasks
             WHERE id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query([task_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(map_plugin_task_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// 函数 `set_plugin_task_enabled`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - task_id: 参数 task_id
    /// - enabled: 参数 enabled
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn set_plugin_task_enabled(&self, task_id: &str, enabled: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE plugin_tasks
             SET enabled = ?1, updated_at = ?2
             WHERE id = ?3",
            (if enabled { 1_i64 } else { 0_i64 }, now_ts(), task_id),
        )?;
        Ok(())
    }

    /// 函数 `update_plugin_task_definition`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - task_id: 参数 task_id
    /// - name: 参数 name
    /// - description: 参数 description
    /// - entrypoint: 参数 entrypoint
    /// - schedule_kind: 参数 schedule_kind
    /// - interval_seconds: 参数 interval_seconds
    /// - enabled: 参数 enabled
    /// - next_run_at: 参数 next_run_at
    /// - task_json: 参数 task_json
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn update_plugin_task_definition(
        &self,
        task_id: &str,
        name: &str,
        description: Option<&str>,
        entrypoint: &str,
        schedule_kind: &str,
        interval_seconds: Option<i64>,
        enabled: bool,
        next_run_at: Option<i64>,
        task_json: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE plugin_tasks
             SET name = ?,
                 description = ?,
                 entrypoint = ?,
                 schedule_kind = ?,
                 interval_seconds = ?,
                 enabled = ?,
                 next_run_at = ?,
                 task_json = ?,
                 updated_at = ?
             WHERE id = ?",
            (
                name,
                description,
                entrypoint,
                schedule_kind,
                interval_seconds,
                if enabled { 1_i64 } else { 0_i64 },
                next_run_at,
                task_json,
                now_ts(),
                task_id,
            ),
        )?;
        Ok(())
    }

    /// 函数 `update_plugin_task_schedule`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - task_id: 参数 task_id
    /// - next_run_at: 参数 next_run_at
    /// - last_run_at: 参数 last_run_at
    /// - last_status: 参数 last_status
    /// - last_error: 参数 last_error
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn update_plugin_task_schedule(
        &self,
        task_id: &str,
        next_run_at: Option<i64>,
        last_run_at: Option<i64>,
        last_status: Option<&str>,
        last_error: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE plugin_tasks
             SET next_run_at = ?1, last_run_at = ?2, last_status = ?3, last_error = ?4, updated_at = ?5
             WHERE id = ?6",
            (next_run_at, last_run_at, last_status, last_error, now_ts(), task_id),
        )?;
        Ok(())
    }

    /// 函数 `list_due_plugin_tasks`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - now: 参数 now
    /// - limit: 参数 limit
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn list_due_plugin_tasks(&self, now: i64, limit: i64) -> Result<Vec<PluginTask>> {
        let normalized_limit = limit.max(1);
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.plugin_id, t.name, t.description, t.entrypoint, t.schedule_kind, t.interval_seconds,
                t.enabled, t.next_run_at, t.last_run_at, t.last_status, t.last_error, t.task_json, t.created_at, t.updated_at
             FROM plugin_tasks t
             INNER JOIN plugin_installs p ON p.plugin_id = t.plugin_id
             WHERE t.enabled = 1 AND p.status = 'enabled' AND t.schedule_kind <> 'manual' AND IFNULL(t.next_run_at, 0) <= ?1
             ORDER BY IFNULL(t.next_run_at, t.created_at) ASC, t.created_at ASC
             LIMIT ?2",
        )?;
        let mut rows = stmt.query(params![now, normalized_limit])?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(map_plugin_task_row(row)?);
        }
        Ok(items)
    }

    /// 函数 `insert_plugin_run_log`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - log: 参数 log
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn insert_plugin_run_log(&self, log: &PluginRunLog) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO plugin_run_logs (
                plugin_id, task_id, run_type, status, started_at, finished_at, duration_ms, output_json, error
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                &log.plugin_id,
                &log.task_id,
                &log.run_type,
                &log.status,
                log.started_at,
                &log.finished_at,
                &log.duration_ms,
                &log.output_json,
                &log.error,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// 函数 `list_plugin_run_logs`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - plugin_id: 参数 plugin_id
    /// - task_id: 参数 task_id
    /// - limit: 参数 limit
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn list_plugin_run_logs(
        &self,
        plugin_id: Option<&str>,
        task_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<PluginRunLog>> {
        let normalized_limit = limit.max(1);
        let mut sql = String::from(
            "SELECT id, plugin_id, task_id, run_type, status, started_at, finished_at, duration_ms, output_json, error
             FROM plugin_run_logs",
        );
        let mut where_clauses = Vec::new();
        let mut params = Vec::new();
        if let Some(plugin_id) = plugin_id {
            where_clauses.push("plugin_id = ?");
            params.push(Value::Text(plugin_id.to_string()));
        }
        if let Some(task_id) = task_id {
            where_clauses.push("task_id = ?");
            params.push(Value::Text(task_id.to_string()));
        }
        if !where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" AND "));
        }
        sql.push_str(" ORDER BY started_at DESC, id DESC LIMIT ?");
        params.push(Value::Integer(normalized_limit));

        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params_from_iter(params.iter()))?;
        let mut items = Vec::new();
        while let Some(row) = rows.next()? {
            items.push(map_plugin_run_log_row(row)?);
        }
        Ok(items)
    }
}

/// 函数 `map_plugin_install_row`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - row: 参数 row
///
/// # 返回
/// 返回函数执行结果
fn map_plugin_install_row(row: &Row<'_>) -> Result<PluginInstall> {
    Ok(PluginInstall {
        plugin_id: row.get(0)?,
        source_url: row.get(1)?,
        name: row.get(2)?,
        version: row.get(3)?,
        description: row.get(4)?,
        author: row.get(5)?,
        homepage_url: row.get(6)?,
        script_url: row.get(7)?,
        script_body: row.get(8)?,
        permissions_json: row.get(9)?,
        manifest_json: row.get(10)?,
        status: row.get(11)?,
        installed_at: row.get(12)?,
        updated_at: row.get(13)?,
        last_run_at: row.get(14)?,
        last_error: row.get(15)?,
    })
}

/// 函数 `map_plugin_task_row`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - row: 参数 row
///
/// # 返回
/// 返回函数执行结果
fn map_plugin_task_row(row: &Row<'_>) -> Result<PluginTask> {
    Ok(PluginTask {
        id: row.get(0)?,
        plugin_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        entrypoint: row.get(4)?,
        schedule_kind: row.get(5)?,
        interval_seconds: row.get(6)?,
        enabled: row.get::<_, i64>(7)? != 0,
        next_run_at: row.get(8)?,
        last_run_at: row.get(9)?,
        last_status: row.get(10)?,
        last_error: row.get(11)?,
        task_json: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

/// 函数 `map_plugin_run_log_row`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - row: 参数 row
///
/// # 返回
/// 返回函数执行结果
fn map_plugin_run_log_row(row: &Row<'_>) -> Result<PluginRunLog> {
    Ok(PluginRunLog {
        id: row.get(0)?,
        plugin_id: row.get(1)?,
        task_id: row.get(2)?,
        run_type: row.get(3)?,
        status: row.get(4)?,
        started_at: row.get(5)?,
        finished_at: row.get(6)?,
        duration_ms: row.get(7)?,
        output_json: row.get(8)?,
        error: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{PluginInstall, PluginTask, Storage};

    /// 函数 `update_plugin_task_definition_updates_interval`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// 无
    ///
    /// # 返回
    /// 无
    #[test]
    fn update_plugin_task_definition_updates_interval() {
        let storage = Storage::open_in_memory().expect("open storage");
        storage.init().expect("init storage");

        let install = PluginInstall {
            plugin_id: "cleanup-banned-accounts".to_string(),
            source_url: Some("builtin://codexmanager".to_string()),
            name: "清理封禁账号".to_string(),
            version: "1.0.0".to_string(),
            description: Some("test".to_string()),
            author: Some("CodexManager".to_string()),
            homepage_url: None,
            script_url: None,
            script_body: "fn run(context) { context }".to_string(),
            permissions_json: serde_json::json!(["accounts:cleanup"]).to_string(),
            manifest_json: serde_json::json!({ "id": "cleanup-banned-accounts" }).to_string(),
            status: "enabled".to_string(),
            installed_at: 1,
            updated_at: 1,
            last_run_at: None,
            last_error: None,
        };
        let task = PluginTask {
            id: "cleanup-banned-accounts::run".to_string(),
            plugin_id: install.plugin_id.clone(),
            name: "手动清理".to_string(),
            description: Some("click".to_string()),
            entrypoint: "run".to_string(),
            schedule_kind: "manual".to_string(),
            interval_seconds: None,
            enabled: true,
            next_run_at: None,
            last_run_at: None,
            last_status: None,
            last_error: None,
            task_json: serde_json::json!({
                "id": "run",
                "name": "手动清理",
                "entrypoint": "run",
                "scheduleKind": "manual",
                "enabled": true
            })
            .to_string(),
            created_at: 1,
            updated_at: 1,
        };

        storage
            .replace_plugin_install(&install, &[task])
            .expect("seed plugin");
        storage
            .update_plugin_task_definition(
                "cleanup-banned-accounts::run",
                "定时自动清理",
                Some("每 60 秒自动清理一次所有封禁账号"),
                "run",
                "interval",
                Some(60),
                true,
                Some(61),
                &serde_json::json!({
                    "id": "run",
                    "name": "定时自动清理",
                    "entrypoint": "run",
                    "scheduleKind": "interval",
                    "intervalSeconds": 60,
                    "enabled": true
                })
                .to_string(),
            )
            .expect("update task");

        let updated = storage
            .find_plugin_task("cleanup-banned-accounts::run")
            .expect("read task")
            .expect("task exists");
        assert_eq!(updated.schedule_kind, "interval");
        assert_eq!(updated.interval_seconds, Some(60));
        assert_eq!(updated.next_run_at, Some(61));
    }

    /// 函数 `list_due_plugin_tasks_returns_enabled_interval_tasks`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// 无
    ///
    /// # 返回
    /// 无
    #[test]
    fn list_due_plugin_tasks_returns_enabled_interval_tasks() {
        let storage = Storage::open_in_memory().expect("open storage");
        storage.init().expect("init storage");

        let install = PluginInstall {
            plugin_id: "cleanup-banned-accounts".to_string(),
            source_url: Some("builtin://codexmanager".to_string()),
            name: "清理封禁账号".to_string(),
            version: "1.0.0".to_string(),
            description: Some("test".to_string()),
            author: Some("CodexManager".to_string()),
            homepage_url: None,
            script_url: None,
            script_body: "fn run(context) { context }".to_string(),
            permissions_json: serde_json::json!(["accounts:cleanup"]).to_string(),
            manifest_json: serde_json::json!({ "id": "cleanup-banned-accounts" }).to_string(),
            status: "enabled".to_string(),
            installed_at: 1,
            updated_at: 1,
            last_run_at: None,
            last_error: None,
        };
        let task = PluginTask {
            id: "cleanup-banned-accounts::run".to_string(),
            plugin_id: install.plugin_id.clone(),
            name: "定时自动清理".to_string(),
            description: Some("auto".to_string()),
            entrypoint: "run".to_string(),
            schedule_kind: "interval".to_string(),
            interval_seconds: Some(60),
            enabled: true,
            next_run_at: Some(10),
            last_run_at: None,
            last_status: None,
            last_error: None,
            task_json: serde_json::json!({
                "id": "run",
                "name": "定时自动清理",
                "entrypoint": "run",
                "scheduleKind": "interval",
                "intervalSeconds": 60,
                "enabled": true
            })
            .to_string(),
            created_at: 1,
            updated_at: 1,
        };

        storage
            .replace_plugin_install(&install, &[task])
            .expect("seed plugin");

        let due = storage
            .list_due_plugin_tasks(100, 10)
            .expect("list due tasks");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, "cleanup-banned-accounts::run");
        assert_eq!(due[0].plugin_id, "cleanup-banned-accounts");
    }
}
