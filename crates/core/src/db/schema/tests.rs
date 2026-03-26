//! schema 迁移测试
//!
//! v13 后 schema 仅保留笔记相关表（知识库五类 + 书签已被 DROP）。

use super::*;

#[test]
fn fresh_db_migrates_to_current() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    assert_eq!(get_user_version(&conn).unwrap(), CURRENT_VERSION);
}

/// 全新库迁移到 CURRENT_VERSION(13) 后：笔记相关表存在，知识库遗留表和书签表已被 DROP。
#[test]
fn v13_schema_has_only_memo_tables() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();

    // 保留表应存在
    for table in [
        "project_categories",
        "tags",
        "memo",
        "memo_tag",
        "memo_version",
        "memo_template",
    ] {
        let n: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{table}'"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "保留表 {table} 应存在");
    }
    // memo_fts 虚拟表应存在
    let fts_n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='memo_fts'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(fts_n, 1, "memo_fts 虚拟表应存在");

    // 书签表应已被 v13 DROP（不存在）
    for table in [
        "bookmarks",
        "bookmark_tag_rel",
        "bookmarks_fts",
    ] {
        let n: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{table}'"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0, "书签表 {table} 应已被 v13 删除");
    }

    // 知识库遗留表应已被 v12 DROP（不存在）
    for table in [
        "projects",
        "snippets",
        "notes",
        "diagrams",
        "templates",
        "assets",
        "shares",
        "languages",
    ] {
        let n: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{table}'"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0, "知识库遗留表 {table} 应已被 v12 删除");
    }
}

/// v13 迁移幂等：表已存在后，CREATE IF EXISTS 应无副作用、不报错。
#[test]
fn v13_migration_is_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    migrate_v13(&conn).unwrap();
}

/// v12 既有库（含书签表）跑 v13 后：笔记表创建，书签表被删除。
#[test]
fn v13_drops_bookmarks_from_v12() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    // 模拟 v12 既有库：直接调 v13
    migrate_v13(&conn).unwrap();
    // 笔记表应存在
    let memo: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='memo'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(memo, 1, "memo 表应存在");
    // 书签表应被删除
    let bm: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='bookmarks'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(bm, 0, "bookmarks 表应被 v13 删除");
}
