//! V10：代码模板（templates）——带 `${name}` 占位符的代码骨架

use crate::CoreResult;
use rusqlite::Connection;

/// V10：代码模板（templates）——与 projects/snippets/notes/diagrams 平级的第五类内容载体
///
/// 存带 `${name}` 占位符的代码模板。模板正文 body 存文件系统（data/templates/…），DB 只存元数据，
/// FTS 由 service 读文件后 sync_template_fts 写入（与 snippets v3 / diagrams v9 一致）。
/// `language` 决定文件扩展名与高亮；`trigger` 是触发词（编辑器补全用，可空）。
/// 注：`trigger` 是 SQL 关键字，作为列名/FTS 列名时用双引号引用。
pub(super) fn migrate_v10(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS templates (
            short_id     TEXT PRIMARY KEY,
            title        TEXT NOT NULL,
            description  TEXT NOT NULL DEFAULT '',
            language     TEXT NOT NULL DEFAULT '',   -- 模板适用语言（决定扩展名/高亮）
            "trigger"    TEXT,                        -- 触发词（可空，编辑器补全用）
            category_id  INTEGER REFERENCES project_categories(id) ON DELETE SET NULL,
            is_favorited INTEGER NOT NULL DEFAULT 0,
            view_count   INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL,
            deleted_at   TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_templates_favor   ON templates(is_favorited);
        CREATE INDEX IF NOT EXISTS idx_templates_created ON templates(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_templates_deleted ON templates(deleted_at);

        -- 模板-标签多对多（复用全库共享的 tags 表）
        CREATE TABLE IF NOT EXISTS template_tag_rel (
            template_id TEXT NOT NULL REFERENCES templates(short_id) ON DELETE CASCADE,
            tag_id      INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (template_id, tag_id)
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS templates_fts USING fts5(
            short_id UNINDEXED,
            title,
            description,
            body,
            tags,
            language,
            "trigger",
            tokenize = 'unicode61'
        );
        "#,
    )?;
    Ok(())
}
