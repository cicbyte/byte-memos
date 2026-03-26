//! service 层冒烟测试：真实临时目录 + SQLite 文件，全部走 ProjectService 公共 API。
//!
//! 覆盖主链路：CRUD、置顶/可见性、软删恢复/彻底删除、搜索（FTS/中文/正则）、
//! 模板、版本快照与回滚、附件（DB 模式）、统计。

use super::*;

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// 每个测试独立的一次性临时目录（app_root 与 data_dir 隔离），返回 (service, 根目录)。
fn temp_service(tag: &str) -> (ProjectService, PathBuf) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("系统时间早于 UNIX_EPOCH")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "byte-memos-svc-{tag}-{}-{nanos}",
        std::process::id()
    ));
    let app_root = root.join("app");
    let data_dir = root.join("app").join("data");
    let svc = ProjectService::open_at(&app_root, &data_dir).expect("open_at 失败");
    (svc, root)
}

/// 测试收尾：先 drop service（释放 SQLite 文件句柄），再尽力清理临时目录。
fn cleanup(svc: ProjectService, root: &PathBuf) {
    drop(svc);
    let _ = std::fs::remove_dir_all(root);
}

fn input(title: &str, content: &str, tags: &[&str], category: Option<&str>) -> AddMemoInput {
    AddMemoInput {
        title: title.into(),
        content: content.into(),
        visibility: "PRIVATE".into(),
        category: category.map(|s| s.into()),
        tags: tags.iter().map(|s| s.to_string()).collect(),
    }
}

// ============ CRUD 主链路 ============

#[test]
fn memo_crud_roundtrip() {
    let (svc, root) = temp_service("crud");
    let m = svc.add_memo(&input("Rust 笔记", "tokio 异步运行时", &["rust", "async"], Some("dev"))).unwrap();
    assert_eq!(m.title, "Rust 笔记");
    assert_eq!(m.tags, vec!["async".to_string(), "rust".to_string()]);
    assert_eq!(m.category.as_deref(), Some("dev"));
    assert!(!m.short_id.is_empty());

    // 覆盖式更新：标题/内容/标签/分类/置顶
    let updated = svc
        .update_memo(
            &m.short_id,
            &UpdateMemoInput {
                title: Some("Rust 进阶".into()),
                content: Some("async/await 原理".into()),
                visibility: None,
                category: Some(Some("advanced".into())),
                tags: Some(vec!["rust".into(), "tokio".into()]),
                pinned: Some(true),
            },
        )
        .unwrap();
    assert_eq!(updated.title, "Rust 进阶");
    assert_eq!(updated.tags.len(), 2);
    assert_eq!(updated.category.as_deref(), Some("advanced"));
    assert!(updated.pinned);

    // 过滤：按标签 / 按分类
    let by_tag = svc
        .list_memos(&MemoFilter { tag: Some("tokio".into()), ..Default::default() })
        .unwrap();
    assert!(by_tag.iter().any(|x| x.short_id == m.short_id));
    let by_cat = svc
        .list_memos(&MemoFilter { category: Some("advanced".into()), ..Default::default() })
        .unwrap();
    assert_eq!(by_cat.len(), 1);

    // 分类列表带 memo_count
    let cats = svc.list_categories().unwrap();
    let advanced = cats.iter().find(|c| c.name == "advanced").unwrap();
    assert_eq!(advanced.memo_count, 1);

    cleanup(svc, &root);
}

#[test]
fn memo_pin_and_visibility() {
    let (svc, root) = temp_service("pin");
    let m = svc.add_memo(&input("t", "c", &[], None)).unwrap();
    assert!(!m.pinned);

    let pinned = svc.toggle_memo_pinned(&m.short_id).unwrap();
    assert!(pinned.pinned);
    let unpinned = svc.toggle_memo_pinned(&m.short_id).unwrap();
    assert!(!unpinned.pinned);

    let pub_memo = svc.set_memo_visibility(&m.short_id, "PUBLIC").unwrap();
    assert_eq!(pub_memo.visibility, "PUBLIC");

    // 置顶过滤
    svc.toggle_memo_pinned(&m.short_id).unwrap();
    let pinned_only = svc
        .list_memos(&MemoFilter { pinned_only: true, ..Default::default() })
        .unwrap();
    assert!(pinned_only.iter().any(|x| x.short_id == m.short_id));

    cleanup(svc, &root);
}

#[test]
fn memo_soft_delete_restore_purge() {
    let (svc, root) = temp_service("trash");
    let a = svc.add_memo(&input("保留", "内容a", &[], None)).unwrap();
    let b = svc.add_memo(&input("待删", "内容b", &[], None)).unwrap();

    svc.delete_memo(&b.short_id).unwrap();
    assert!(svc.get_memo(&b.short_id).is_err(), "软删后 get 应报 NotFound");

    let trashed = svc.list_trashed().unwrap();
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].short_id, b.short_id);

    // 恢复
    svc.restore_memo(&b.short_id).unwrap();
    assert!(svc.get_memo(&b.short_id).is_ok());
    assert!(svc.list_trashed().unwrap().is_empty());

    // 彻底删除
    svc.delete_memo(&b.short_id).unwrap();
    svc.purge_memo(&b.short_id).unwrap();
    assert!(svc.list_trashed().unwrap().is_empty());
    assert!(svc.get_memo(&a.short_id).is_ok(), "彻底删除不应影响其他笔记");

    // 清空回收站
    svc.delete_memo(&a.short_id).unwrap();
    svc.empty_trash().unwrap();
    assert!(svc.list_trashed().unwrap().is_empty());

    cleanup(svc, &root);
}

// ============ 搜索（FTS / 中文兜底 / 正则）============

#[test]
fn memo_search_fts_chinese() {
    let (svc, root) = temp_service("search");
    svc.add_memo(&input("Tokio Runtime", "async runtime internals", &[], None)).unwrap();
    svc.add_memo(&input("数据库设计笔记", "索引与范式", &[], None)).unwrap();

    // 英文走 FTS
    let en = svc.search_memos("tokio", 10, 0).unwrap();
    assert!(en.iter().any(|h| h.title == "Tokio Runtime"));
    // 中文走 LIKE 兜底
    let zh = svc.search_memos("数据库", 10, 0).unwrap();
    assert!(zh.iter().any(|h| h.title == "数据库设计笔记"));

    cleanup(svc, &root);
}

#[test]
fn memo_search_regex() {
    let (svc, root) = temp_service("regex");
    svc.add_memo(&input("版本记录", "发布版本 v1.2.3，下一个 v1.3.0", &[], None)).unwrap();
    svc.add_memo(&input("无关笔记", "普通内容", &[], None)).unwrap();

    let hits = svc.search_regex(r"v\d+\.\d+\.\d+", 10, 0).unwrap();
    assert_eq!(hits.len(), 1, "只应命中版本记录笔记");
    assert_eq!(hits[0].title, "版本记录");
    assert!(hits[0].snippet.contains("v1.2.3"));

    // 无匹配
    assert!(svc.search_regex(r"^\d+$", 10, 0).unwrap().is_empty());
    // 非法正则报错而非 panic
    assert!(svc.search_regex("(unclosed", 10, 0).is_err());
    // limit 生效
    for i in 0..5 {
        svc.add_memo(&input(&format!("regex-{i}"), "pattern-12345", &[], None)).unwrap();
    }
    assert!(svc.search_regex("pattern-\\d+", 2, 0).unwrap().len() <= 2);

    cleanup(svc, &root);
}

// ============ 模板 ============

#[test]
fn template_lifecycle() {
    let (svc, root) = temp_service("tpl");
    svc.create_template("会议纪要", "## 会议：{{标题}}", "标准模板").unwrap();

    let list = svc.list_templates().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "会议纪要");

    // 从模板建笔记
    let m = svc.create_memo_from_template("会议纪要", Some("周会")).unwrap();
    assert_eq!(m.title, "周会");
    assert_eq!(m.content, "## 会议：{{标题}}");

    // 更新模板
    svc.update_template("会议纪要", Some("## 会议（新）"), Some("v2")).unwrap();
    let updated = svc.get_template("会议纪要").unwrap().unwrap();
    assert_eq!(updated.content, "## 会议（新）");

    // 笔记存为模板
    let tid = svc.save_memo_as_template(&m.short_id, "快照模板", "来自笔记").unwrap();
    assert!(tid > 0);
    assert_eq!(svc.list_templates().unwrap().len(), 2);

    // 删除模板（不影响由它创建的笔记）
    svc.delete_template("会议纪要").unwrap();
    assert!(svc.get_memo(&m.short_id).is_ok());

    cleanup(svc, &root);
}

// ============ 版本快照与回滚 ============

#[test]
fn version_snapshot_and_restore() {
    let (svc, root) = temp_service("ver");
    let m = svc.add_memo(&input("草稿", "第一版内容", &[], None)).unwrap();

    // 内容变更 → 自动快照旧内容
    svc.update_memo(
        &m.short_id,
        &UpdateMemoInput { content: Some("第二版内容".into()), ..Default::default() },
    )
    .unwrap();
    let versions = svc.list_versions(&m.short_id).unwrap();
    assert_eq!(versions.len(), 1, "一次内容变更应产生一个快照");
    assert_eq!(versions[0].content, "第一版内容");

    // 回滚：内容恢复，且历史只增不减
    let restored = svc.restore_version(&m.short_id, versions[0].id).unwrap();
    assert_eq!(restored.content, "第一版内容");
    let versions2 = svc.list_versions(&m.short_id).unwrap();
    assert_eq!(versions2.len(), 2, "回滚本身也创建新版本记录");

    cleanup(svc, &root);
}

// ============ 附件（DB 存储模式）============

#[test]
fn attachment_db_lifecycle() {
    let (svc, root) = temp_service("att");
    let m = svc.add_memo(&input("带附件", "内容", &[], None)).unwrap();

    let id = svc
        .upload_attachment_db(&m.short_id, "数据.txt", "text/plain", b"hello attachment")
        .unwrap();
    let list = svc.list_attachments(&m.short_id).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].filename, "数据.txt");

    let content = svc.get_attachment_content(id).unwrap().unwrap();
    assert_eq!(content, b"hello attachment");

    svc.delete_attachment(id).unwrap();
    assert!(svc.list_attachments(&m.short_id).unwrap().is_empty());

    // 附件挂在不存在的笔记上应报错
    assert!(svc.upload_attachment_db("NOPE0000", "x.txt", "text/plain", b"x").is_err());

    cleanup(svc, &root);
}

// ============ 附件（FILE 存储模式 + 资源库总览 + purge 级联）============

#[test]
fn attachment_file_bytes_upload_and_purge_cascade() {
    let (svc, root) = temp_service("attfile");
    let m = svc.add_memo(&input("文件附件", "内容", &[], None)).unwrap();

    // 字节上传（桌面端路径）：落盘 + 记 FILE 记录
    let id = svc
        .upload_attachment_bytes(&m.short_id, "图片.png", "image/png", b"\x89PNG-bytes")
        .unwrap();
    let list = svc.list_attachments(&m.short_id).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].storage_type, "FILE");

    // 统一内容读取（FILE 模式读磁盘）
    let (att, bytes) = svc.read_attachment_content(id).unwrap().unwrap();
    assert_eq!(att.filename, "图片.png");
    assert_eq!(bytes, b"\x89PNG-bytes");

    // 资源库总览：备忘录存在 → 非 orphan
    let all = svc.list_all_attachments().unwrap();
    assert_eq!(all.len(), 1);
    assert!(!all[0].orphan);

    // purge 备忘录 → 附件记录与磁盘文件一并清理
    svc.purge_memo(&m.short_id).unwrap();
    let all = svc.list_all_attachments().unwrap();
    assert!(all.is_empty(), "purge 应级联清理附件记录");
    let file = root.join("attachments").join(&m.short_id).join("图片.png");
    assert!(!file.exists(), "purge 应删除磁盘附件文件");

    // 桌面上传到不存在的笔记应报错
    assert!(svc.upload_attachment_bytes("NOPE0000", "x.txt", "", b"x").is_err());

    cleanup(svc, &root);
}

// ============ 统计 ============

#[test]
fn stats_overview_counts() {
    let (svc, root) = temp_service("stats");
    svc.add_memo(&input("a", "内容a", &["rust"], Some("dev"))).unwrap();
    let b = svc.add_memo(&input("b", "内容b", &["rust"], None)).unwrap();
    svc.toggle_memo_pinned(&b.short_id).unwrap();
    svc.delete_memo(&b.short_id).unwrap();

    let s = svc.stats_overview().unwrap();
    assert_eq!(s.memo_count, 1, "回收站内笔记不计入活跃数");
    assert_eq!(s.tag_count, 1);
    assert_eq!(s.category_count, 1);
    assert_eq!(s.trashed_count, 1);

    cleanup(svc, &root);
}
