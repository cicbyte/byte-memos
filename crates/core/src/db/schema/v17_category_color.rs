//! V17：分类主题色
//!
//! - project_categories 增加 color 列（可空，TEXT 存 hex 色值）
//! - 工具软件个性化：分类色点贯穿侧栏 / 备忘录列表 / 分类管理页
//! - 新旧库统一走本迁移（新库建表后仍会按序执行到 v17）

use crate::CoreResult;
use rusqlite::Connection;

pub fn migrate_v17(conn: &Connection) -> CoreResult<()> {
    // 幂等守卫：遗留库版本冲突补跑时列可能已存在（ALTER 无 IF NOT EXISTS 语义）
    if !super::column_exists(conn, "project_categories", "color")? {
        conn.execute_batch("ALTER TABLE project_categories ADD COLUMN color TEXT;")?;
        tracing::info!("V17: project_categories 增加 color 列");
    }
    Ok(())
}
