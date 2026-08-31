use rusqlite::{params_from_iter, types::Value, Result};
use std::collections::HashMap;

use super::{Event, Storage};

impl Storage {
    /// 函数 `insert_event`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - event: 参数 event
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn insert_event(&self, event: &Event) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (account_id, type, message, created_at) VALUES (?1, ?2, ?3, ?4)",
            (
                &event.account_id,
                &event.event_type,
                &event.message,
                event.created_at,
            ),
        )?;
        Ok(())
    }

    /// 函数 `event_count`
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
    pub fn event_count(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(1) FROM events", [], |row| row.get(0))
    }

    /// 函数 `latest_account_status_reasons`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - account_ids: 参数 account_ids
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn latest_account_status_reasons(
        &self,
        account_ids: &[String],
    ) -> Result<HashMap<String, String>> {
        if account_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders = vec!["?"; account_ids.len()].join(", ");
        let sql = format!(
            "WITH ranked AS (
                SELECT
                    account_id,
                    message,
                    ROW_NUMBER() OVER (
                        PARTITION BY account_id
                        ORDER BY created_at DESC, id DESC
                    ) AS rn
                FROM events
                WHERE type = 'account_status_update'
                  AND account_id IN ({placeholders})
            )
            SELECT account_id, message
            FROM ranked
            WHERE rn = 1"
        );

        let params = account_ids
            .iter()
            .map(|account_id| Value::Text(account_id.clone()))
            .collect::<Vec<_>>();
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(params_from_iter(params))?;
        let mut out = HashMap::new();
        while let Some(row) = rows.next()? {
            let account_id: String = row.get(0)?;
            let message: String = row.get(1)?;
            if let Some(reason) = extract_status_reason_from_event_message(&message) {
                out.insert(account_id, reason.to_string());
            }
        }
        Ok(out)
    }
}

/// 函数 `extract_status_reason_from_event_message`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - message: 参数 message
///
/// # 返回
/// 返回函数执行结果
fn extract_status_reason_from_event_message(message: &str) -> Option<&str> {
    let marker = " reason=";
    let start = message.find(marker)? + marker.len();
    let reason = message.get(start..)?.trim();
    if reason.is_empty() {
        None
    } else {
        Some(reason)
    }
}

#[cfg(test)]
#[path = "tests/events_tests.rs"]
mod tests;
