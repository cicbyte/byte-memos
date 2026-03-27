//! V18：AI 供应商配置表
//!
//! - ai_providers：多提供商（openai-compatible / anthropic / gemini / ollama），
//!   api_key 仅 Rust 侧 DPAPI 解密使用，永不回显前端
//! - 启用为单选语义（应用层保证），表结构不约束
//! - 新旧库统一走本迁移（CREATE TABLE IF NOT EXISTS 幂等）

use crate::CoreResult;
use rusqlite::Connection;

pub fn migrate_v18(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ai_providers (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         VARCHAR(100) NOT NULL,
            provider_key VARCHAR(80) NOT NULL UNIQUE,
            base_url     VARCHAR(500) NOT NULL DEFAULT '',
            api_key      VARCHAR(500) NOT NULL DEFAULT '',
            model        VARCHAR(200) NOT NULL DEFAULT '',
            max_tokens   INTEGER NOT NULL DEFAULT 4096,
            temperature  REAL NOT NULL DEFAULT 0.7,
            is_enabled   INTEGER NOT NULL DEFAULT 0,
            sort_order   INTEGER NOT NULL DEFAULT 0,
            created_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at   DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
    )?;
    tracing::info!("V18: ai_providers 表就绪");
    Ok(())
}
