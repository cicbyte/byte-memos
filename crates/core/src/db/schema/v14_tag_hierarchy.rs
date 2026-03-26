//! V14：标签层级结构支持
//!
//! - 给 tags 表添加 parent_id 字段（支持层级标签如 #工作/项目A）
//! - 给 tags 表添加 color 字段（可选颜色）

use crate::CoreResult;
use rusqlite::Connection;

pub fn migrate_v14(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        "
        -- 给 tags 表添加 parent_id 字段（支持层级）
        ALTER TABLE tags ADD COLUMN parent_id INTEGER REFERENCES tags(id) ON DELETE CASCADE;

        -- 给 tags 表添加 color 字段（可选颜色）
        ALTER TABLE tags ADD COLUMN color TEXT;

        -- 创建索引加速层级查询
        CREATE INDEX IF NOT EXISTS idx_tags_parent ON tags(parent_id);
        ",
    )?;
    tracing::info!("V14: 标签层级结构支持完成");
    Ok(())
}
