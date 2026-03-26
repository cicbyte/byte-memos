//! V2/V3：代码片段表建表 + 去 code 列（代码体移到文件系统）

use crate::CoreResult;
use rusqlite::Connection;

/// V2：代码片段（snippets）——轻量、无文件结构的单条代码记录，与 projects 平级独立存储
pub(super) fn migrate_v2(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        r#"
        -- ============ 代码片段 ============
        CREATE TABLE IF NOT EXISTS snippets (
            short_id     TEXT PRIMARY KEY,
            title        TEXT NOT NULL,
            description  TEXT NOT NULL DEFAULT '',
            code         TEXT NOT NULL,
            language     TEXT NOT NULL DEFAULT '',   -- 直接存语言名，单语言
            category_id  INTEGER REFERENCES project_categories(id) ON DELETE SET NULL,
            is_favorited INTEGER NOT NULL DEFAULT 0,
            view_count   INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_snippets_favor   ON snippets(is_favorited);
        CREATE INDEX IF NOT EXISTS idx_snippets_created ON snippets(created_at DESC);

        -- ============ 片段-标签多对多（复用全库共享的 tags 表）============
        CREATE TABLE IF NOT EXISTS snippet_tag_rel (
            snippet_id TEXT NOT NULL REFERENCES snippets(short_id) ON DELETE CASCADE,
            tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (snippet_id, tag_id)
        );

        -- ============ 片段全文索引 ============
        CREATE VIRTUAL TABLE IF NOT EXISTS snippets_fts USING fts5(
            short_id UNINDEXED,
            title,
            description,
            code,
            tags,
            language,
            tokenize = 'unicode61'
        );
        "#,
    )?;
    Ok(())
}

/// V3：snippets 表去掉 code 列——代码体移到文件系统（~/.cicbyte/byte-link/snippets/{id}.{ext}）
///
/// 迁移策略（DDL-only，文件搬运由 service 层协调）：
/// 1. 把存量 (short_id, code, language) 导出到 snippets_code_backup 临时表
/// 2. DROP 旧 snippets + snippets_fts
/// 3. 重建 snippets（无 code 列）+ snippets_fts
/// 4. backup 表保留，service.open_at 读取后落盘再删除
///
/// 注意：v3 只在 v2→v3 时执行；全新库（current=0）跑完 v1+v2+v3 后 snippets 本就无 code 列，
///       backup 表为空，service 扫描时跳过。
pub(super) fn migrate_v3(conn: &Connection) -> CoreResult<()> {
    // 1. 导出存量 code（仅当 snippets 表存在且有 code 列时）
    let snippets_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='snippets'",
        [],
        |r| r.get(0),
    )?;
    if snippets_exists == 1 {
        // 检测 snippets 是否还有 code 列（v2 建的有，全新库跑 v2 后也有——但全新库没数据）
        let has_code_col = {
            let cols: Vec<String> = conn
                .prepare("PRAGMA table_info(snippets)")?
                .query_map([], |r| r.get::<_, String>(1))?
                .filter_map(|r| r.ok())
                .collect();
            cols.iter().any(|c| c == "code")
        };
        if has_code_col {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS snippets_code_backup (
                    short_id TEXT PRIMARY KEY,
                    code     TEXT NOT NULL,
                    language TEXT NOT NULL DEFAULT ''
                );
                INSERT OR REPLACE INTO snippets_code_backup(short_id, code, language)
                SELECT short_id, code, language FROM snippets;
                DROP TABLE snippets;
                DROP TABLE snippets_fts;
                CREATE TABLE snippets (
                    short_id     TEXT PRIMARY KEY,
                    title        TEXT NOT NULL,
                    description  TEXT NOT NULL DEFAULT '',
                    language     TEXT NOT NULL DEFAULT '',
                    category_id  INTEGER REFERENCES project_categories(id) ON DELETE SET NULL,
                    is_favorited INTEGER NOT NULL DEFAULT 0,
                    view_count   INTEGER NOT NULL DEFAULT 0,
                    created_at   TEXT NOT NULL,
                    updated_at   TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_snippets_favor   ON snippets(is_favorited);
                CREATE INDEX IF NOT EXISTS idx_snippets_created ON snippets(created_at DESC);
                CREATE TABLE IF NOT EXISTS snippet_tag_rel (
                    snippet_id TEXT NOT NULL REFERENCES snippets(short_id) ON DELETE CASCADE,
                    tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                    PRIMARY KEY (snippet_id, tag_id)
                );
                CREATE VIRTUAL TABLE snippets_fts USING fts5(
                    short_id UNINDEXED, title, description, code, tags, language,
                    tokenize='unicode61'
                );
            ",
            )?;
        }
    }
    Ok(())
}
