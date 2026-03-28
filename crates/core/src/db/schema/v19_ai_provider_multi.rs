//! V19：放开 ai_providers.provider_key 唯一约束
//!
//! 内置常用供应商（GLM / DeepSeek / 小米 MiMo 等）多为 openai-compatible
//! 类型，同 key 多行是常态（Cherry Studio 模式：内置商常驻列表）。
//! SQLite 无法直接 DROP 约束，按标准流程重建表再回填。

use crate::CoreResult;
use rusqlite::Connection;

pub fn migrate_v19(conn: &Connection) -> CoreResult<()> {
    // 幂等守卫：仅当现表 DDL 仍带 UNIQUE 时才重建
    let ddl: String = conn
        .query_row(
            "SELECT COALESCE(sql, '') FROM sqlite_master WHERE type='table' AND name='ai_providers'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();
    if !ddl.contains("UNIQUE") {
        return Ok(());
    }
    conn.execute_batch(
        "CREATE TABLE ai_providers_v19 (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            name         VARCHAR(100) NOT NULL,
            provider_key VARCHAR(80) NOT NULL,
            base_url     VARCHAR(500) NOT NULL DEFAULT '',
            api_key      VARCHAR(500) NOT NULL DEFAULT '',
            model        VARCHAR(200) NOT NULL DEFAULT '',
            max_tokens   INTEGER NOT NULL DEFAULT 4096,
            temperature  REAL NOT NULL DEFAULT 0.7,
            is_enabled   INTEGER NOT NULL DEFAULT 0,
            sort_order   INTEGER NOT NULL DEFAULT 0,
            created_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at   DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        INSERT INTO ai_providers_v19
            (id, name, provider_key, base_url, api_key, model, max_tokens, temperature, is_enabled, sort_order, created_at, updated_at)
        SELECT id, name, provider_key, base_url, api_key, model, max_tokens, temperature, is_enabled, sort_order, created_at, updated_at
          FROM ai_providers;
        DROP TABLE ai_providers;
        ALTER TABLE ai_providers_v19 RENAME TO ai_providers;",
    )?;
    tracing::info!("V19: ai_providers 重建，provider_key 唯一约束已移除");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    #[test]
    fn v19_drops_unique_and_keeps_rows() -> CoreResult<()> {
        let conn = Connection::open_in_memory().expect("内存库");
        run_migrations(&conn)?;
        // v18 建的同 key 第二行在 v19 后应可插入
        conn.execute(
            "INSERT INTO ai_providers (name, provider_key) VALUES ('a', 'openai-compatible')",
            [],
        )?;
        conn.execute(
            "INSERT INTO ai_providers (name, provider_key) VALUES ('b', 'openai-compatible')",
            [],
        )?;
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ai_providers WHERE provider_key = 'openai-compatible'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(n, 2);
        Ok(())
    }
}
