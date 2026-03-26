//! V8：资产（assets）——图片内容寻址存储的元数据表

use crate::CoreResult;
use rusqlite::Connection;

/// V8：资产（assets）——图片内容寻址存储的元数据表
///
/// 主键 md5（文件内容 MD5），相同内容去重。物理文件路径由 md5+ext 复算（见 storage），
/// 不入库，保证备份/迁移后路径可重建。deleted_at 实现资产回收站（软删除，文件保留）。
pub(super) fn migrate_v8(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS assets (
            md5           TEXT PRIMARY KEY,
            ext           TEXT NOT NULL,
            size          INTEGER NOT NULL,
            width         INTEGER,
            height        INTEGER,
            original_name TEXT NOT NULL DEFAULT '',
            thumb_exists  INTEGER NOT NULL DEFAULT 0,
            created_at    TEXT NOT NULL,
            deleted_at    TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_assets_deleted ON assets(deleted_at);
        "#,
    )?;
    Ok(())
}
