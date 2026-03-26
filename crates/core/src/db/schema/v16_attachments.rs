//! V16：附件管理表
//!
//! - 创建 attachment 表（笔记附件）
//! - 支持可配置存储方式（DB 内存储 / 文件系统存储）

use crate::CoreResult;
use rusqlite::Connection;

pub fn migrate_v16(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        "
        -- 创建 attachment 表（笔记附件）
        CREATE TABLE IF NOT EXISTS attachment (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            memo_short_id TEXT NOT NULL REFERENCES memo(short_id) ON DELETE CASCADE,
            filename    TEXT    NOT NULL,
            file_path   TEXT    NOT NULL,
            file_size   INTEGER NOT NULL DEFAULT 0,
            mime_type   TEXT    NOT NULL DEFAULT 'application/octet-stream',
            storage_type TEXT   NOT NULL CHECK(storage_type IN ('DB', 'FILE')) DEFAULT 'FILE',
            blob        BLOB,
            created_at  TEXT    NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_attachment_memo ON attachment(memo_short_id);
        CREATE INDEX IF NOT EXISTS idx_attachment_filename ON attachment(filename);
        ",
    )?;
    tracing::info!("V16: 附件管理表创建完成");
    Ok(())
}
