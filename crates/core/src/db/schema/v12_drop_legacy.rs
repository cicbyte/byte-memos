//! v12：移除知识库遗留表
//!
//! byte-link 转型为书签应用，知识库五类（projects / snippets / notes / diagrams /
//! templates）+ 资产（assets）+ 分享（shares）+ 语言（languages）表不再需要。
//!
//! 保留：bookmarks / bookmark_tag_rel / bookmarks_fts / tags / project_categories /
//! snapshots（backup 用）。
//!
//! 注意：既有库执行此迁移会丢失知识库数据，属 M6 清理预期行为（PRD §2.3）。
//! 先删关联表 / FTS / 备份表（子表），再删主表，避免外键引用悬空。

use crate::CoreResult;
use rusqlite::Connection;

pub fn migrate_v12(conn: &Connection) -> CoreResult<()> {
    conn.execute_batch(
        "
        -- 项目
        DROP TABLE IF EXISTS project_tag_rel;
        DROP TABLE IF EXISTS project_lang_rel;
        DROP TABLE IF EXISTS projects_fts;
        DROP TABLE IF EXISTS projects;
        -- 片段
        DROP TABLE IF EXISTS snippet_tag_rel;
        DROP TABLE IF EXISTS snippets_fts;
        DROP TABLE IF EXISTS snippets_code_backup;
        DROP TABLE IF EXISTS snippets;
        -- 笔记
        DROP TABLE IF EXISTS note_tag_rel;
        DROP TABLE IF EXISTS notes_fts;
        DROP TABLE IF EXISTS notes;
        -- 图表
        DROP TABLE IF EXISTS diagram_tag_rel;
        DROP TABLE IF EXISTS diagrams_fts;
        DROP TABLE IF EXISTS diagrams;
        -- 模板
        DROP TABLE IF EXISTS template_tag_rel;
        DROP TABLE IF EXISTS templates_fts;
        DROP TABLE IF EXISTS templates;
        -- 资产 / 分享 / 语言
        DROP TABLE IF EXISTS assets;
        DROP TABLE IF EXISTS shares;
        DROP TABLE IF EXISTS languages;
        ",
    )?;
    Ok(())
}
