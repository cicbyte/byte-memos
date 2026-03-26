//! V13：创建 memo 笔记表，替换 bookmarks 书签表
//!
//! - 创建 memo 表（笔记核心实体）
//! - 创建 memo_tag 关联表（多对多标签）
//! - 创建 memo_fts FTS5 虚拟表（全文搜索）
//! - 创建 memo_version 表（版本历史）
//! - 创建 memo_template 表（笔记模板）
//! - DROP bookmarks 相关表

use crate::CoreResult;
use rusqlite::Connection;

pub fn migrate_v13(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        "
        -- 1. 创建 memo 表
        CREATE TABLE IF NOT EXISTS memo (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            short_id    TEXT    NOT NULL UNIQUE,
            title       TEXT    NOT NULL DEFAULT '',
            content     TEXT    NOT NULL DEFAULT '',
            visibility  TEXT    NOT NULL CHECK(visibility IN ('PRIVATE','PROTECTED','PUBLIC')) DEFAULT 'PRIVATE',
            pinned      INTEGER NOT NULL DEFAULT 0,
            category_id INTEGER REFERENCES project_categories(id) ON DELETE SET NULL,
            created_at  TEXT    NOT NULL,
            updated_at  TEXT    NOT NULL,
            deleted_at  TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_memo_short_id ON memo(short_id);
        CREATE INDEX IF NOT EXISTS idx_memo_category ON memo(category_id) WHERE deleted_at IS NULL;
        CREATE INDEX IF NOT EXISTS idx_memo_deleted ON memo(deleted_at);
        CREATE INDEX IF NOT EXISTS idx_memo_pinned ON memo(pinned) WHERE deleted_at IS NULL;
        CREATE INDEX IF NOT EXISTS idx_memo_visibility ON memo(visibility) WHERE deleted_at IS NULL;

        -- 2. 创建 memo_tag 关联表
        CREATE TABLE IF NOT EXISTS memo_tag (
            memo_id TEXT NOT NULL REFERENCES memo(short_id) ON DELETE CASCADE,
            tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (memo_id, tag_id)
        );

        CREATE INDEX IF NOT EXISTS idx_memo_tag_tag ON memo_tag(tag_id);

        -- 3. 创建 memo_fts FTS5 虚拟表（内部内容，手动同步）
        CREATE VIRTUAL TABLE IF NOT EXISTS memo_fts USING fts5(
            title,
            content,
            tags,
            tokenize='unicode61'
        );

        -- FTS 同步触发器（内部内容 FTS5 表用标准 DELETE / INSERT）
        CREATE TRIGGER IF NOT EXISTS memo_ai AFTER INSERT ON memo BEGIN
            INSERT INTO memo_fts(rowid, title, content, tags)
            VALUES (new.id, new.title, new.content, '');
        END;

        CREATE TRIGGER IF NOT EXISTS memo_ad AFTER DELETE ON memo BEGIN
            DELETE FROM memo_fts WHERE rowid = old.id;
        END;

        CREATE TRIGGER IF NOT EXISTS memo_au AFTER UPDATE ON memo BEGIN
            DELETE FROM memo_fts WHERE rowid = old.id;
            INSERT INTO memo_fts(rowid, title, content, tags)
            VALUES (new.id, new.title, new.content, '');
        END;

        -- 4. 创建 memo_version 表（版本历史）
        CREATE TABLE IF NOT EXISTS memo_version (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            memo_id    INTEGER NOT NULL REFERENCES memo(id) ON DELETE CASCADE,
            content    TEXT    NOT NULL,
            created_at TEXT    NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_memo_version_memo ON memo_version(memo_id);

        -- 5. 创建 memo_template 表（笔记模板）
        CREATE TABLE IF NOT EXISTS memo_template (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT    NOT NULL UNIQUE,
            content     TEXT    NOT NULL DEFAULT '',
            description TEXT    NOT NULL DEFAULT '',
            created_at  TEXT    NOT NULL,
            updated_at  TEXT    NOT NULL
        );

        -- 6. 删除旧 bookmarks 相关表
        DROP TRIGGER IF EXISTS bookmarks_ai;
        DROP TRIGGER IF EXISTS bookmarks_ad;
        DROP TRIGGER IF EXISTS bookmarks_au;
        DROP TABLE IF EXISTS bookmarks_fts;
        DROP TABLE IF EXISTS bookmark_tag_rel;
        DROP TABLE IF EXISTS bookmarks;
        ",
    )?;
    tracing::info!("V13: memo 表结构创建完成，bookmarks 已移除");
    Ok(())
}
