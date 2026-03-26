//! V11：书签（bookmarks）——byte-link 核心领域实体
//!
//! 与 projects/snippets/notes/templates/diagrams 平级，但面向书签：
//! - short_id 主键（复用全库 trash / FTS / 标签机制）
//! - url 业务唯一键：用「部分唯一索引」`WHERE deleted_at IS NULL` 保证仅活跃行唯一，
//!   软删除（进回收站）后 url 可被重新添加
//! - description 短文本直接存 DB，FTS 用 `INSERT ... SELECT` 同步 DB 列（无需落盘内容文件）
//! - is_archived 归档（书签特有，区别于 deleted_at 回收站）
//! - 复用全库共享的 tags / project_categories 表

use crate::CoreResult;
use rusqlite::Connection;

pub(super) fn migrate_v11(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS bookmarks (
            short_id     TEXT PRIMARY KEY,
            title        TEXT NOT NULL,
            url          TEXT NOT NULL,             -- 业务键；唯一性由下方部分唯一索引保证（仅活跃行）
            description  TEXT NOT NULL DEFAULT '',
            category_id  INTEGER REFERENCES project_categories(id) ON DELETE SET NULL,
            favicon_path TEXT NOT NULL DEFAULT '',
            visit_count  INTEGER NOT NULL DEFAULT 0,
            is_archived  INTEGER NOT NULL DEFAULT 0,
            is_favorited INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL,
            deleted_at   TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_bookmarks_favor    ON bookmarks(is_favorited);
        CREATE INDEX IF NOT EXISTS idx_bookmarks_archived ON bookmarks(is_archived);
        CREATE INDEX IF NOT EXISTS idx_bookmarks_created  ON bookmarks(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_bookmarks_deleted  ON bookmarks(deleted_at);
        CREATE INDEX IF NOT EXISTS idx_bookmarks_category ON bookmarks(category_id);
        -- 仅活跃行（deleted_at IS NULL）内 url 唯一：软删除后 url 可复用
        CREATE UNIQUE INDEX IF NOT EXISTS idx_bookmarks_url_unique ON bookmarks(url) WHERE deleted_at IS NULL;

        -- 书签-标签多对多（复用全库共享的 tags 表）
        CREATE TABLE IF NOT EXISTS bookmark_tag_rel (
            bookmark_id TEXT NOT NULL REFERENCES bookmarks(short_id) ON DELETE CASCADE,
            tag_id      INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (bookmark_id, tag_id)
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS bookmarks_fts USING fts5(
            short_id UNINDEXED,
            title,
            url,
            description,
            tags,
            tokenize = 'unicode61'
        );
        "#,
    )?;
    Ok(())
}
