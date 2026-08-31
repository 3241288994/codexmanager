use rusqlite::params;

use super::{ConversationBinding, Storage};

impl Storage {
    /// 函数 `get_conversation_binding`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - platform_key_hash: 参数 platform_key_hash
    /// - conversation_id: 参数 conversation_id
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn get_conversation_binding(
        &self,
        platform_key_hash: &str,
        conversation_id: &str,
    ) -> rusqlite::Result<Option<ConversationBinding>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                platform_key_hash,
                conversation_id,
                account_id,
                thread_epoch,
                thread_anchor,
                status,
                last_model,
                last_switch_reason,
                created_at,
                updated_at,
                last_used_at
             FROM conversation_bindings
             WHERE platform_key_hash = ?1
               AND conversation_id = ?2
             LIMIT 1",
        )?;
        let mut rows = stmt.query([platform_key_hash, conversation_id])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(ConversationBinding {
                platform_key_hash: row.get(0)?,
                conversation_id: row.get(1)?,
                account_id: row.get(2)?,
                thread_epoch: row.get(3)?,
                thread_anchor: row.get(4)?,
                status: row.get(5)?,
                last_model: row.get(6)?,
                last_switch_reason: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                last_used_at: row.get(10)?,
            }));
        }
        Ok(None)
    }

    /// 函数 `upsert_conversation_binding`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - binding: 参数 binding
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn upsert_conversation_binding(
        &self,
        binding: &ConversationBinding,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO conversation_bindings (
                platform_key_hash,
                conversation_id,
                account_id,
                thread_epoch,
                thread_anchor,
                status,
                last_model,
                last_switch_reason,
                created_at,
                updated_at,
                last_used_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
             )
             ON CONFLICT(platform_key_hash, conversation_id) DO UPDATE SET
                account_id = excluded.account_id,
                thread_epoch = excluded.thread_epoch,
                thread_anchor = excluded.thread_anchor,
                status = excluded.status,
                last_model = excluded.last_model,
                last_switch_reason = excluded.last_switch_reason,
                updated_at = excluded.updated_at,
                last_used_at = excluded.last_used_at",
            params![
                &binding.platform_key_hash,
                &binding.conversation_id,
                &binding.account_id,
                binding.thread_epoch,
                &binding.thread_anchor,
                &binding.status,
                &binding.last_model,
                &binding.last_switch_reason,
                binding.created_at,
                binding.updated_at,
                binding.last_used_at,
            ],
        )?;
        Ok(())
    }

    /// 函数 `touch_conversation_binding`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - platform_key_hash: 参数 platform_key_hash
    /// - conversation_id: 参数 conversation_id
    /// - account_id: 参数 account_id
    /// - last_model: 参数 last_model
    /// - touched_at: 参数 touched_at
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn touch_conversation_binding(
        &self,
        platform_key_hash: &str,
        conversation_id: &str,
        account_id: &str,
        last_model: Option<&str>,
        touched_at: i64,
    ) -> rusqlite::Result<bool> {
        let updated = self.conn.execute(
            "UPDATE conversation_bindings
             SET last_model = ?4,
                 last_used_at = ?5,
                 updated_at = ?5
             WHERE platform_key_hash = ?1
               AND conversation_id = ?2
               AND account_id = ?3",
            params![
                platform_key_hash,
                conversation_id,
                account_id,
                last_model,
                touched_at,
            ],
        )?;
        Ok(updated > 0)
    }

    /// 函数 `delete_conversation_binding`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - platform_key_hash: 参数 platform_key_hash
    /// - conversation_id: 参数 conversation_id
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn delete_conversation_binding(
        &self,
        platform_key_hash: &str,
        conversation_id: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM conversation_bindings
             WHERE platform_key_hash = ?1
               AND conversation_id = ?2",
            params![platform_key_hash, conversation_id],
        )?;
        Ok(())
    }

    /// 函数 `delete_conversation_bindings_for_account`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - account_id: 参数 account_id
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn delete_conversation_bindings_for_account(
        &self,
        account_id: &str,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM conversation_bindings
             WHERE account_id = ?1",
            [account_id],
        )?;
        Ok(())
    }

    /// 函数 `delete_stale_conversation_bindings`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - older_than_ts: 参数 older_than_ts
    ///
    /// # 返回
    /// 返回函数执行结果
    pub fn delete_stale_conversation_bindings(
        &self,
        older_than_ts: i64,
    ) -> rusqlite::Result<usize> {
        self.conn.execute(
            "DELETE FROM conversation_bindings
             WHERE last_used_at < ?1",
            [older_than_ts],
        )
    }
}

#[cfg(test)]
#[path = "tests/conversation_bindings_tests.rs"]
mod tests;
