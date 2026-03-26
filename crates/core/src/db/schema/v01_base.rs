//! V1：初始 schema——分类/标签/语言/项目表 + FTS5 + 同步触发器

use crate::CoreResult;
use rusqlite::Connection;

/// V1：初始 schema——全部表 + FTS5 + 同步触发器
pub(super) fn migrate_v1(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        r#"
        -- ============ 分类 ============
        CREATE TABLE IF NOT EXISTS project_categories (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL UNIQUE
        );

        -- ============ 标签 ============
        CREATE TABLE IF NOT EXISTS tags (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL UNIQUE
        );

        -- ============ 语言 ============
        CREATE TABLE IF NOT EXISTS languages (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL UNIQUE
        );

        -- ============ 项目（核心表）============
        CREATE TABLE IF NOT EXISTS projects (
            short_id     TEXT PRIMARY KEY,
            title        TEXT NOT NULL,
            description  TEXT NOT NULL DEFAULT '',
            category_id  INTEGER REFERENCES project_categories(id) ON DELETE SET NULL,
            status       TEXT NOT NULL DEFAULT 'normal',
            is_favorited INTEGER NOT NULL DEFAULT 0,
            view_count   INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL,
            source_path  TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_projects_category ON projects(category_id);
        CREATE INDEX IF NOT EXISTS idx_projects_favor    ON projects(is_favorited);
        CREATE INDEX IF NOT EXISTS idx_projects_created  ON projects(created_at DESC);

        -- ============ 多对多关联 ============
        CREATE TABLE IF NOT EXISTS project_tag_rel (
            project_id TEXT NOT NULL REFERENCES projects(short_id) ON DELETE CASCADE,
            tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (project_id, tag_id)
        );
        CREATE TABLE IF NOT EXISTS project_lang_rel (
            project_id TEXT NOT NULL REFERENCES projects(short_id) ON DELETE CASCADE,
            lang_id    INTEGER NOT NULL REFERENCES languages(id) ON DELETE CASCADE,
            PRIMARY KEY (project_id, lang_id)
        );

        -- ============ 版本快照（F8，表结构预留，功能 M3）============
        CREATE TABLE IF NOT EXISTS snapshots (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id     TEXT NOT NULL REFERENCES projects(short_id) ON DELETE CASCADE,
            created_at     TEXT NOT NULL,
            note           TEXT NOT NULL DEFAULT '',
            manifest_path  TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_snapshots_project ON snapshots(project_id);

        -- ============ FTS5 全文索引（external content 模式）============
        -- 索引 title / description / tags / category / languages
        CREATE VIRTUAL TABLE IF NOT EXISTS projects_fts USING fts5(
            short_id UNINDEXED,
            title,
            description,
            tags,
            category,
            languages,
            tokenize = 'unicode61'
        );

        -- ============ FTS 同步触发器 ============
        -- 注：tags/category/languages 由 service 层在写库后用聚合字符串显式同步
        --     （关联表无单行聚合列，无法纯触发器完成）。此处提供 projects 行级同步，
        --     关联字段的 FTS 刷新由 service 调 sync_fts_for_project() 完成。

        CREATE TRIGGER IF NOT EXISTS projects_fts_ai AFTER INSERT ON projects BEGIN
            INSERT INTO projects_fts(short_id, title, description, tags, category, languages)
            VALUES (new.short_id, new.title, new.description, '', '', '');
        END;
        CREATE TRIGGER IF NOT EXISTS projects_fts_ad AFTER DELETE ON projects BEGIN
            DELETE FROM projects_fts WHERE short_id = old.short_id;
        END;
        CREATE TRIGGER IF NOT EXISTS projects_fts_au AFTER UPDATE ON projects BEGIN
            DELETE FROM projects_fts WHERE short_id = old.short_id;
            INSERT INTO projects_fts(short_id, title, description, tags, category, languages)
            VALUES (new.short_id, new.title, new.description,
                (SELECT COALESCE(group_concat(t.name, ' '),'') FROM project_tag_rel r
                   JOIN tags t ON t.id = r.tag_id WHERE r.project_id = new.short_id),
                (SELECT COALESCE(c.name,'') FROM project_categories c
                   WHERE c.id = new.category_id),
                (SELECT COALESCE(group_concat(l.name, ' '),'') FROM project_lang_rel r
                   JOIN languages l ON l.id = r.lang_id WHERE r.project_id = new.short_id));
        END;
        "#,
    )?;
    Ok(())
}
