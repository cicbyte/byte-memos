//! V5：回收站软删除列；V6：移除置顶（is_pinned）列

use crate::CoreResult;
use rusqlite::Connection;

/// V5：回收站——给三张主表加 `deleted_at` 软删除列
///
/// 语义：NULL = 正常态，非空 = 已进回收站（存 ISO8601 删除时间）。
/// 软删除是 UPDATE，不触发关联表 CASCADE，tags/category 关系天然保留；
/// 恢复只需把 deleted_at 置 NULL。
///
/// 用 ALTER TABLE ADD COLUMN 幂等性差（重复添加报错），故逐表检测列是否存在。
pub(super) fn migrate_v5(conn: &Connection) -> CoreResult<()> {
    for table in ["projects", "snippets", "notes"] {
        let has_col = {
            let cols: Vec<String> = conn
                .prepare(&format!("PRAGMA table_info({table})"))?
                .query_map([], |r| r.get::<_, String>(1))?
                .filter_map(|r| r.ok())
                .collect();
            cols.iter().any(|c| c == "deleted_at")
        };
        if !has_col {
            conn.execute_batch(&format!(
                "ALTER TABLE {table} ADD COLUMN deleted_at TEXT"
            ))?;
        }
        conn.execute_batch(&format!(
            "CREATE INDEX IF NOT EXISTS idx_{table}_deleted ON {table}(deleted_at)"
        ))?;
    }
    Ok(())
}

/// V6：移除"置顶"功能——删除 projects/snippets/notes 的 is_pinned 列与索引
///
/// SQLite 3.35+ 支持 `DROP COLUMN`。列不存在时跳过（全新库 v1-v5 建表后 is_pinned 仍在，
/// 此迁移会执行；列已被删除的库跳过）。索引 DROP IF EXISTS 保证幂等。
pub(super) fn migrate_v6(conn: &Connection) -> CoreResult<()> {
    for table in ["projects", "snippets", "notes"] {
        let has_col = {
            let cols: Vec<String> = conn
                .prepare(&format!("PRAGMA table_info({table})"))?
                .query_map([], |r| r.get::<_, String>(1))?
                .filter_map(|r| r.ok())
                .collect();
            cols.iter().any(|c| c == "is_pinned")
        };
        if !has_col {
            continue;
        }
        conn.execute_batch(&format!(
            "DROP INDEX IF EXISTS idx_{table}_pinned;
             ALTER TABLE {table} DROP COLUMN is_pinned;"
        ))?;
    }
    Ok(())
}
