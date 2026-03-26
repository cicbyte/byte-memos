//! V15：笔记关联关系表
//!
//! - 创建 memo_relation 表（笔记间双向引用）
//! - 支持 REFERENCE 和 COMMENT 两种关联类型

use crate::CoreResult;
use rusqlite::Connection;

pub fn migrate_v15(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        "
        -- 创建 memo_relation 表（笔记间关联）
        CREATE TABLE IF NOT EXISTS memo_relation (
            source_memo_id TEXT NOT NULL REFERENCES memo(short_id) ON DELETE CASCADE,
            target_memo_id TEXT NOT NULL REFERENCES memo(short_id) ON DELETE CASCADE,
            relation_type  TEXT NOT NULL CHECK(relation_type IN ('REFERENCE', 'COMMENT')) DEFAULT 'REFERENCE',
            created_at     TEXT NOT NULL,
            PRIMARY KEY (source_memo_id, target_memo_id, relation_type)
        );

        CREATE INDEX IF NOT EXISTS idx_memo_relation_source ON memo_relation(source_memo_id);
        CREATE INDEX IF NOT EXISTS idx_memo_relation_target ON memo_relation(target_memo_id);
        ",
    )?;
    tracing::info!("V15: 笔记关联关系表创建完成");
    Ok(())
}
