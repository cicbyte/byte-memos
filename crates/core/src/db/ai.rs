//! AI 供应商数据访问（ai_providers 表）
//!
//! feature = "ai" 下编译。SQL 唯一接触面在本模块；
//! api_key 加解密在 service 层完成（本模块只存取原样字符串）。

use crate::CoreResult;
use rusqlite::params;

/// 供应商行（api_key 为存储原样：dpapi: 密文或存量明文）
#[derive(Debug, Clone)]
pub struct AiProviderRow {
    pub id: i64,
    pub name: String,
    pub provider_key: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: i64,
    pub temperature: f64,
    pub is_enabled: bool,
    pub sort_order: i64,
}

impl super::Database {
    pub fn ai_provider_list(&self) -> CoreResult<Vec<AiProviderRow>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, provider_key, base_url, api_key, model, max_tokens, temperature, \
             is_enabled, sort_order FROM ai_providers ORDER BY sort_order ASC, id ASC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(AiProviderRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    provider_key: r.get(2)?,
                    base_url: r.get(3)?,
                    api_key: r.get(4)?,
                    model: r.get(5)?,
                    max_tokens: r.get(6)?,
                    temperature: r.get(7)?,
                    is_enabled: r.get::<_, i64>(8)? != 0,
                    sort_order: r.get(9)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 添加供应商。api_key 须已加密；is_enabled 由调用方决定（首条自动启用）。
    /// provider_key 允许重复（内置预设多为 openai-compatible，同 key 多行是常态）。
    pub fn ai_provider_add(
        &self,
        name: &str,
        provider_key: &str,
        base_url: &str,
        api_key_encrypted: &str,
        model: &str,
        max_tokens: i64,
        temperature: f64,
        is_enabled: bool,
    ) -> CoreResult<i64> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO ai_providers (name, provider_key, base_url, api_key, model, max_tokens, temperature, is_enabled) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![name, provider_key, base_url, api_key_encrypted, model, max_tokens, temperature, is_enabled as i64],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 内置预设种子：按名称去重插入（存在同名行则跳过）。
    /// api_key 存空串（unprotect 明文穿透语义兼容），启用状态一律关闭。
    /// 返回是否实际插入。
    pub fn ai_provider_seed_if_missing(
        &self,
        name: &str,
        provider_key: &str,
        base_url: &str,
        model: &str,
        sort_order: i64,
    ) -> CoreResult<bool> {
        let conn = self.lock()?;
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ai_providers WHERE name = ?",
            params![name],
            |r| r.get(0),
        )?;
        if exists > 0 {
            return Ok(false);
        }
        conn.execute(
            "INSERT INTO ai_providers (name, provider_key, base_url, api_key, model, sort_order, is_enabled) \
             VALUES (?, ?, ?, '', ?, ?, 0)",
            params![name, provider_key, base_url, model, sort_order],
        )?;
        Ok(true)
    }

    /// 更新供应商。api_key_encrypted 为空串表示保留原值（调用方解出原值后回传密文）。
    /// is_enabled=true 时事务内停用其余（单选语义）。
    pub fn ai_provider_update(
        &self,
        id: i64,
        name: &str,
        base_url: &str,
        api_key_encrypted: &str,
        model: &str,
        max_tokens: i64,
        temperature: f64,
        is_enabled: bool,
    ) -> CoreResult<()> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        if is_enabled {
            tx.execute(
                "UPDATE ai_providers SET is_enabled = 0, updated_at = CURRENT_TIMESTAMP WHERE id <> ?",
                params![id],
            )?;
        }
        let sql = if api_key_encrypted.is_empty() {
            "UPDATE ai_providers SET name=?, base_url=?, model=?, max_tokens=?, temperature=?, \
             is_enabled=?, updated_at=CURRENT_TIMESTAMP WHERE id=?"
        } else {
            "UPDATE ai_providers SET name=?, base_url=?, api_key=?, model=?, max_tokens=?, temperature=?, \
             is_enabled=?, updated_at=CURRENT_TIMESTAMP WHERE id=?"
        };
        let affected = if api_key_encrypted.is_empty() {
            tx.execute(sql, params![name, base_url, model, max_tokens, temperature, is_enabled as i64, id])?
        } else {
            tx.execute(sql, params![name, base_url, api_key_encrypted, model, max_tokens, temperature, is_enabled as i64, id])?
        };
        if affected == 0 {
            return Err(crate::CoreError::NotFound(format!("AI 供应商 id={id}")));
        }
        tx.commit()?;
        Ok(())
    }

    pub fn ai_provider_delete(&self, id: i64) -> CoreResult<()> {
        let conn = self.lock()?;
        let affected = conn.execute("DELETE FROM ai_providers WHERE id = ?", params![id])?;
        if affected == 0 {
            return Err(crate::CoreError::NotFound(format!("AI 供应商 id={id}")));
        }
        Ok(())
    }

    /// 读取存储原样的 api_key（service 层据此实现「空=保留原值」）
    pub fn ai_provider_api_key(&self, id: i64) -> CoreResult<Option<String>> {
        let conn = self.lock()?;
        let v: Option<String> = conn
            .query_row(
                "SELECT api_key FROM ai_providers WHERE id = ?",
                params![id],
                |r| r.get(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(v)
    }

    /// 按 id 取完整行（api_key 原样）
    pub fn ai_provider_by_id(&self, id: i64) -> CoreResult<Option<AiProviderRow>> {
        let conn = self.lock()?;
        let v = conn
            .query_row(
                "SELECT id, name, provider_key, base_url, api_key, model, max_tokens, temperature, \
                 is_enabled, sort_order FROM ai_providers WHERE id = ?",
                params![id],
                |r| {
                    Ok(AiProviderRow {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        provider_key: r.get(2)?,
                        base_url: r.get(3)?,
                        api_key: r.get(4)?,
                        model: r.get(5)?,
                        max_tokens: r.get(6)?,
                        temperature: r.get(7)?,
                        is_enabled: r.get::<_, i64>(8)? != 0,
                        sort_order: r.get(9)?,
                    })
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(v)
    }

    /// 唯一启用中的供应商（无则 Ok(None)）
    pub fn ai_provider_enabled(&self) -> CoreResult<Option<AiProviderRow>> {
        let conn = self.lock()?;
        let v = conn
            .query_row(
                "SELECT id, name, provider_key, base_url, api_key, model, max_tokens, temperature, \
                 is_enabled, sort_order FROM ai_providers WHERE is_enabled = 1 ORDER BY sort_order ASC, id ASC LIMIT 1",
                [],
                |r| {
                    Ok(AiProviderRow {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        provider_key: r.get(2)?,
                        base_url: r.get(3)?,
                        api_key: r.get(4)?,
                        model: r.get(5)?,
                        max_tokens: r.get(6)?,
                        temperature: r.get(7)?,
                        is_enabled: r.get::<_, i64>(8)? != 0,
                        sort_order: r.get(9)?,
                    })
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(v)
    }
}
