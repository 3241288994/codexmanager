use rusqlite::params;

use super::Storage;

impl Storage {
    /// 函数 `list_app_settings`
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
    pub fn list_app_settings(&self) -> rusqlite::Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT key, value
             FROM app_settings
             ORDER BY key ASC",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    /// 函数 `get_app_setting`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - key: 参数 key
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn get_app_setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT value
             FROM app_settings
             WHERE key = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query([key])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(row.get(0)?));
        }
        Ok(None)
    }

    /// 函数 `set_app_setting`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - key: 参数 key
    /// - value: 参数 value
    /// - updated_at: 参数 updated_at
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn set_app_setting(&self, key: &str, value: &str, updated_at: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO app_settings (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
               value = excluded.value,
               updated_at = excluded.updated_at",
            params![key, value, updated_at],
        )?;
        Ok(())
    }

    /// 函数 `delete_app_setting`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - key: 参数 key
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn delete_app_setting(&self, key: &str) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM app_settings WHERE key = ?1", [key])?;
        Ok(())
    }
}
