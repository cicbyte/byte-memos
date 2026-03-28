//! V20：ai_providers 增加模型列表列
//!
//! models_json：该供应商可用模型集合（JSON 字符串数组），
//! 由「获取模型列表」或手动添加维护；`model` 字段仍是默认模型。

use crate::CoreResult;
use rusqlite::Connection;

pub fn migrate_v20(conn: &Connection) -> CoreResult<()> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('ai_providers') WHERE name = 'models_json'",
        [],
        |r| r.get(0),
    )?;
    if n > 0 {
        return Ok(());
    }
    conn.execute_batch(
        "ALTER TABLE ai_providers ADD COLUMN models_json TEXT NOT NULL DEFAULT '[]';",
    )?;
    tracing::info!("V20: ai_providers.models_json 列就绪");
    Ok(())
}
