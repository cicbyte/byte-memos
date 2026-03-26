//! db 模块的单元测试（笔记单一类型）。

use super::*;
use super::common::build_fts_query;

fn mem_db() -> Database {
    let db = Database::open_in_memory_for_test();
    assert!(db.is_ok(), "open_in_memory 失败: {:?}", db.err());
    db.unwrap()
}

#[test]
fn fts_query_builder() {
    assert_eq!(build_fts_query("mcp server"), "\"mcp\"* \"server\"*");
    assert_eq!(build_fts_query(""), "");
}

#[test]
fn upsert_tag_is_idempotent() {
    let db = mem_db();
    let id1 = db.upsert_tag("rust").unwrap();
    let id2 = db.upsert_tag("rust").unwrap();
    assert_eq!(id1, id2);
}

// ============ memo 基础 ============

fn add_memo(db: &Database, sid: &str, title: &str, content: &str, tags: &[&str]) {
    let m = Memo {
        short_id: sid.into(),
        title: title.into(),
        content: content.into(),
        visibility: "PRIVATE".into(),
        pinned: false,
        category_id: None,
        category: None,
        tags: vec![],
        created_at: "2026-01-01T00:00:00Z".into(),
        updated_at: "2026-01-01T00:00:00Z".into(),
        deleted_at: None,
    };
    db.insert_memo(&m).unwrap();
    let tag_ids: Vec<i64> = tags.iter().map(|t| db.upsert_tag(t).unwrap()).collect();
    db.set_memo_tags(sid, &tag_ids).unwrap();
    db.sync_memo_fts(sid).unwrap();
}

#[test]
fn memo_id_collision_detection() {
    let db = mem_db();
    assert!(!db.memo_id_exists("ABCD1234").unwrap());
    add_memo(&db, "ABCD1234", "t", "内容", &[]);
    assert!(db.memo_id_exists("ABCD1234").unwrap());
}

#[test]
fn short_id_exists_checks_memos() {
    let db = mem_db();
    assert!(!db.short_id_exists("SHRTID01").unwrap());
    add_memo(&db, "SHRTID01", "t", "内容", &[]);
    assert!(db.short_id_exists("SHRTID01").unwrap());
}

#[test]
fn trashed_count_only_memos() {
    let db = mem_db();
    add_memo(&db, "TRS00001", "待删", "内容", &[]);
    assert_eq!(db.trashed_count().unwrap(), 0);
    db.trash_memo("TRS00001").unwrap();
    assert_eq!(db.trashed_count().unwrap(), 1);
}

#[test]
fn list_trashed_memos_only_soft_deleted() {
    let db = mem_db();
    add_memo(&db, "ACT00001", "活跃", "内容", &[]);
    add_memo(&db, "TRS00002", "已删", "内容", &[]);
    db.trash_memo("TRS00002").unwrap();
    let trashed = db.list_trashed_memos().unwrap();
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].short_id, "TRS00002");
    assert_eq!(trashed[0].item_type, ItemType::Memo);
}

// ============ search_all（FTS + 中文 LIKE 兜底）============

#[test]
fn search_all_english_via_fts() {
    let db = mem_db();
    add_memo(&db, "MMEN0001", "Tokio Runtime", "相关内容", &[]);
    let hits = db.search_all("tokio", 9, 0).unwrap();
    assert!(hits.iter().any(|h| h.short_id == "MMEN0001"));
    // FTS 命中 rank 应为负数（bm25）
    let hit = hits.iter().find(|h| h.short_id == "MMEN0001").unwrap();
    assert!(hit.rank < 0.0, "FTS bm25 命中 rank 应为负数，实际: {}", hit.rank);
}

#[test]
fn search_all_chinese_via_like_fallback() {
    let db = mem_db();
    add_memo(&db, "MMCN0001", "数据库设计笔记", "相关内容", &[]);
    let hits = db.search_all("数据库", 9, 0).unwrap();
    assert!(hits.iter().any(|h| h.short_id == "MMCN0001" && h.item_type == ItemType::Memo));
}

#[test]
fn search_all_excludes_soft_deleted() {
    let db = mem_db();
    add_memo(&db, "DEL00001", "可搜索笔记", "内容", &[]);
    db.trash_memo("DEL00001").unwrap();
    let hits = db.search_all("可搜索", 9, 0).unwrap();
    assert!(!hits.iter().any(|h| h.short_id == "DEL00001"), "回收站项不应被搜索到");
}

#[test]
fn search_all_empty_query_returns_empty() {
    let db = mem_db();
    assert!(db.search_all("", 9, 0).unwrap().is_empty());
    assert!(db.search_all("   ", 9, 0).unwrap().is_empty());
}

#[test]
fn search_all_respects_limit() {
    let db = mem_db();
    for i in 0..10 {
        add_memo(
            &db,
            &format!("LM{T:07}", T = i),
            &format!("limit_test_{}", i),
            &format!("内容_{}", i),
            &[],
        );
    }
    let hits = db.search_all("limit_test", 3, 0).unwrap();
    assert!(hits.len() <= 3, "总结果不应超过 limit，实际: {}", hits.len());
}

// ============ stats_overview ============

#[test]
fn stats_overview_counts_memos() {
    let db = mem_db();
    add_memo(&db, "STT00001", "a", "内容a", &["rust"]);
    add_memo(&db, "STT00002", "b", "内容b", &["rust", "io"]);
    db.upsert_category("tools").unwrap();
    // 置顶一个
    let m = Memo {
        short_id: "STT00003".into(),
        title: "c".into(),
        content: "内容c".into(),
        visibility: "PRIVATE".into(),
        pinned: true,
        category_id: None,
        category: None,
        tags: vec![],
        created_at: "2026-01-01T00:00:00Z".into(),
        updated_at: "2026-01-01T00:00:00Z".into(),
        deleted_at: None,
    };
    db.insert_memo(&m).unwrap();

    let s = db.stats_overview().unwrap();
    assert_eq!(s.memo_count, 3);
    assert_eq!(s.tag_count, 2); // rust, io
    assert_eq!(s.pinned_count, 1);
    assert_eq!(s.trashed_count, 0);
}

#[test]
fn tag_stats_aggregates_memo_refs_only() {
    let db = mem_db();
    add_memo(&db, "TG000001", "a", "内容a", &["rust", "io"]);
    add_memo(&db, "TG000002", "b", "内容b", &["rust"]);
    let stats = db.tag_stats(100).unwrap();
    let rust = stats.iter().find(|t| t.tag == "rust").unwrap();
    assert_eq!(rust.count, 2);
    let io = stats.iter().find(|t| t.tag == "io").unwrap();
    assert_eq!(io.count, 1);
    // 软删除后计数减少
    db.trash_memo("TG000002").unwrap();
    let stats2 = db.tag_stats(100).unwrap();
    let rust2 = stats2.iter().find(|t| t.tag == "rust").unwrap();
    assert_eq!(rust2.count, 1);
}

// ============ list_categories（memo_count）============

#[test]
fn list_categories_counts_memos() {
    let db = mem_db();
    let cid = db.upsert_category("tools").unwrap();
    // 在 tools 分类下建 2 个备忘录
    for sid in ["CAT00001", "CAT00002"] {
        let m = Memo {
            short_id: sid.into(),
            title: sid.into(),
            content: format!("{sid} 内容"),
            visibility: "PRIVATE".into(),
            pinned: false,
            category_id: Some(cid),
            category: None,
            tags: vec![],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            deleted_at: None,
        };
        db.insert_memo(&m).unwrap();
    }
    // 不在分类下的备忘录
    add_memo(&db, "NOPE0001", "x", "内容x", &[]);

    let cats = db.list_categories().unwrap();
    let tools = cats.iter().find(|c| c.name == "tools").unwrap();
    assert_eq!(tools.memo_count, 2);
}
