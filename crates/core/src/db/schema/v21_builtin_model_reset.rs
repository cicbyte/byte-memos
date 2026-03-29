//! V21：清空内置供应商的预置默认模型
//!
//! 决策：模型一律由用户「获取模型列表 → 手动选择」，内置行不再预置默认模型。
//! 仅清空「model 等于旧 seed 值」的内置供应商行（用户显式改过的模型保留）；
//! 新种子的内置行 model 本就为空（见 service::ai::BUILTIN_PROVIDERS）。

use crate::CoreResult;
use rusqlite::Connection;

/// 旧 seed 预置的默认模型（按行匹配，避免误伤用户显式选择）
const LEGACY_SEED_MODELS: &[&str] = &[
    "glm-4.6",
    "deepseek-chat",
    "kimi-k2-turbo-preview",
    "qwen-plus",
    "deepseek-ai/DeepSeek-V3",
    "mimo-v2.5",
    "gpt-4o-mini",
    "claude-sonnet-4-5",
    "gemini-2.5-flash",
    "qwen3:8b",
];

pub fn migrate_v21(conn: &Connection) -> CoreResult<()> {
    let placeholders = LEGACY_SEED_MODELS.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "UPDATE ai_providers SET model = '' WHERE model IN ({placeholders});",
        placeholders = placeholders
    );
    let list = LEGACY_SEED_MODELS.to_vec();
    let n = conn.execute(&sql, rusqlite::params_from_iter(list))?;
    if n > 0 {
        tracing::info!("V21: 清空了 {n} 行内置供应商的预置默认模型");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::run_migrations;

    #[test]
    fn v21_clears_seed_models_but_keeps_user_choice() -> CoreResult<()> {
        let conn = Connection::open_in_memory()?;
        run_migrations(&conn)?;
        // 内置行带旧 seed 默认模型 → 应被清空
        conn.execute(
            "INSERT INTO ai_providers (name, provider_key, base_url, api_key, model, models_json, max_tokens, temperature, is_enabled, sort_order)
             VALUES ('GLM 智谱', 'openai-compatible', 'https://x', '', 'glm-4.6', '[]', 0, 0.0, 0, 0)",
            [],
        )?;
        // 用户显式设置的模型 → 应保留
        conn.execute(
            "INSERT INTO ai_providers (name, provider_key, base_url, api_key, model, models_json, max_tokens, temperature, is_enabled, sort_order)
             VALUES ('DeepSeek', 'openai-compatible', 'https://y', '', 'deepseek-reasoner', '[]', 0, 0.0, 0, 1)",
            [],
        )?;
        migrate_v21(&conn)?;
        let glm: String = conn.query_row(
            "SELECT model FROM ai_providers WHERE name = 'GLM 智谱'", [], |r| r.get(0),
        )?;
        let ds: String = conn.query_row(
            "SELECT model FROM ai_providers WHERE name = 'DeepSeek'", [], |r| r.get(0),
        )?;
        assert_eq!(glm, "", "seed 默认模型应被清空");
        assert_eq!(ds, "deepseek-reasoner", "用户显式选择的模型应保留");
        // 幂等：再跑一次不报错
        migrate_v21(&conn)?;
        Ok(())
    }
}
