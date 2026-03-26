//! V7：分享令牌表

use crate::CoreResult;
use rusqlite::Connection;

/// V7：分享——为 project/snippet/note 生成对外只读分享令牌
///
/// token 与目标 item 的关联由本表维护；目标 item 被软删/删除时，本表不会自动清理
/// （item_type/item_id 异构，无法 FK CASCADE），由 service 层在 resolve 时校验存活。
pub(super) fn migrate_v7(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS shares (
            token          TEXT PRIMARY KEY,
            item_type      TEXT NOT NULL,    -- 'project' | 'snippet' | 'note'
            item_id        TEXT NOT NULL,    -- 对应实体的 short_id
            created_at     TEXT NOT NULL,
            expires_at     TEXT,             -- NULL = 永久有效
            view_count     INTEGER NOT NULL DEFAULT 0,
            last_viewed_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_shares_item ON shares(item_type, item_id);
        "#,
    )?;
    Ok(())
}
