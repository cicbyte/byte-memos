//! V9：图表（diagrams）——Mermaid/SVG 源码图

use crate::CoreResult;
use rusqlite::Connection;

/// V9：图表（diagrams）——与 projects/snippets/notes 平级的第四类内容载体
///
/// 存 Mermaid/SVG 源码图。源码体存文件系统（data/diagrams/…），DB 只存元数据，
/// FTS 由 service 读文件后 sync_diagram_fts 写入（与 snippets v3 / notes v4 一致）。
/// kind 字段区分 mermaid/svg，决定渲染方式与文件扩展名（mmd/svg）。
pub(super) fn migrate_v9(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS diagrams (
            short_id     TEXT PRIMARY KEY,
            title        TEXT NOT NULL,
            description  TEXT NOT NULL DEFAULT '',
            kind         TEXT NOT NULL DEFAULT 'mermaid',   -- 'mermaid' | 'svg'
            category_id  INTEGER REFERENCES project_categories(id) ON DELETE SET NULL,
            is_favorited INTEGER NOT NULL DEFAULT 0,
            view_count   INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL,
            deleted_at   TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_diagrams_favor   ON diagrams(is_favorited);
        CREATE INDEX IF NOT EXISTS idx_diagrams_created ON diagrams(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_diagrams_deleted ON diagrams(deleted_at);

        -- 图表-标签多对多（复用全库共享的 tags 表）
        CREATE TABLE IF NOT EXISTS diagram_tag_rel (
            diagram_id TEXT NOT NULL REFERENCES diagrams(short_id) ON DELETE CASCADE,
            tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (diagram_id, tag_id)
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS diagrams_fts USING fts5(
            short_id UNINDEXED,
            title,
            description,
            source,
            tags,
            kind,
            tokenize = 'unicode61'
        );
        "#,
    )?;
    Ok(())
}
