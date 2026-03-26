//! V4：笔记（notes）——轻量 Markdown 文档，内容体存文件系统

use crate::CoreResult;
use rusqlite::Connection;

/// V4：笔记（notes）——轻量 Markdown 文档，内容体存文件系统
pub(super) fn migrate_v4(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS notes (
            short_id     TEXT PRIMARY KEY,
            title        TEXT NOT NULL,
            description  TEXT NOT NULL DEFAULT '',
            category_id  INTEGER REFERENCES project_categories(id) ON DELETE SET NULL,
            is_favorited INTEGER NOT NULL DEFAULT 0,
            view_count   INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_notes_favor   ON notes(is_favorited);
        CREATE INDEX IF NOT EXISTS idx_notes_created ON notes(created_at DESC);

        CREATE TABLE IF NOT EXISTS note_tag_rel (
            note_id TEXT NOT NULL REFERENCES notes(short_id) ON DELETE CASCADE,
            tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (note_id, tag_id)
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
            short_id UNINDEXED,
            title,
            description,
            content,
            tags,
            tokenize = 'unicode61'
        );
        "#,
    )?;
    Ok(())
}
