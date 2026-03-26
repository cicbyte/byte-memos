//! memo 子命令：添加 / 列表 / 详情 / 更新 / 删除 / 搜索 / 标签 / 置顶 / 可见性
//!
//! byte-memos 的核心领域。title 和 content 至少填一个。

use clap::{Args, Subcommand};

use byte_memos_core::model::{AddMemoInput, MemoFilter, UpdateMemoInput};

use super::open_service;
use crate::format;

#[derive(Subcommand, Debug)]
pub enum MemoCommand {
    /// 添加备忘录
    Add(MemoAddArgs),
    /// 列出备忘录
    List(MemoListArgs),
    /// 查看备忘录详情
    Get(MemoGetArgs),
    /// 更新备忘录（未提供的字段保留原值）
    Update(MemoUpdateArgs),
    /// 删除备忘录（移入回收站）
    Delete(MemoDeleteArgs),
    /// 全文搜索备忘录
    Search(MemoSearchArgs),
    /// 打 / 去标签
    Tag(MemoTagArgs),
    /// 切换置顶状态
    Pin(MemoPinArgs),
    /// 设置可见性
    Visibility(MemoVisibilityArgs),
    /// 创建笔记引用（双向链接）
    Link(MemoLinkArgs),
    /// 删除笔记引用
    Unlink(MemoUnlinkArgs),
    /// 查看笔记关联关系
    Relations(MemoRelationsArgs),
    /// 查看笔记版本历史
    History(MemoHistoryArgs),
    /// 回滚到指定版本
    RestoreVersion(MemoRestoreVersionArgs),
}

#[derive(Args, Debug)]
pub struct MemoAddArgs {
    /// 标题
    #[arg(long)]
    pub title: Option<String>,
    /// 内容
    #[arg(long)]
    pub content: Option<String>,
    /// 可见性：PRIVATE / PROTECTED / PUBLIC
    #[arg(long, default_value = "PRIVATE")]
    pub visibility: String,
    /// 分类
    #[arg(long)]
    pub category: Option<String>,
    /// 标签（可多次）
    #[arg(long = "tag", value_name = "TAG")]
    pub tags: Vec<String>,
}

#[derive(Args, Debug)]
pub struct MemoListArgs {
    #[arg(long)]
    pub category: Option<String>,
    #[arg(long)]
    pub tag: Option<String>,
    /// 仅列出已置顶
    #[arg(long)]
    pub pinned: bool,
    #[arg(long, default_value = "50")]
    pub limit: i64,
}

#[derive(Args, Debug)]
pub struct MemoGetArgs {
    /// 备忘录 shortId
    pub short_id: String,
}

#[derive(Args, Debug)]
pub struct MemoUpdateArgs {
    /// 备忘录 shortId
    pub short_id: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub content: Option<String>,
    /// 可见性：PRIVATE / PROTECTED / PUBLIC
    #[arg(long)]
    pub visibility: Option<String>,
    #[arg(long)]
    pub category: Option<String>,
    /// 标签（覆盖；不传则保留原标签）
    #[arg(long = "tag", value_name = "TAG")]
    pub tags: Vec<String>,
    /// 置顶
    #[arg(long)]
    pub pin: bool,
    /// 取消置顶
    #[arg(long)]
    pub unpin: bool,
}

#[derive(Args, Debug)]
pub struct MemoDeleteArgs {
    /// 备忘录 shortId
    pub short_id: String,
    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct MemoSearchArgs {
    /// 搜索词
    pub query: String,
    #[arg(long, default_value = "20")]
    pub limit: i64,
    #[arg(long, default_value = "0")]
    pub offset: i64,
}

#[derive(Args, Debug)]
pub struct MemoTagArgs {
    /// 备忘录 shortId
    pub short_id: String,
    /// 添加标签（可多次）
    #[arg(long = "add", value_name = "TAG")]
    pub add: Vec<String>,
    /// 移除标签（可多次）
    #[arg(long = "remove", value_name = "TAG")]
    pub remove: Vec<String>,
}

#[derive(Args, Debug)]
pub struct MemoPinArgs {
    /// 备忘录 shortId
    pub short_id: String,
}

#[derive(Args, Debug)]
pub struct MemoVisibilityArgs {
    /// 备忘录 shortId
    pub short_id: String,
    /// 可见性：PRIVATE / PROTECTED / PUBLIC
    pub visibility: String,
}

#[derive(Args, Debug)]
pub struct MemoLinkArgs {
    /// 源笔记 shortId
    pub source_id: String,
    /// 目标笔记 shortId
    pub target_id: String,
}

#[derive(Args, Debug)]
pub struct MemoUnlinkArgs {
    /// 源笔记 shortId
    pub source_id: String,
    /// 目标笔记 shortId
    pub target_id: String,
}

#[derive(Args, Debug)]
pub struct MemoRelationsArgs {
    /// 备忘录 shortId
    pub short_id: String,
}

#[derive(Args, Debug)]
pub struct MemoHistoryArgs {
    /// 备忘录 shortId
    pub short_id: String,
}

#[derive(Args, Debug)]
pub struct MemoRestoreVersionArgs {
    /// 备忘录 shortId
    pub short_id: String,
    /// 版本ID
    pub version_id: i64,
}

pub async fn run_memo_add(args: MemoAddArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let title = args.title.unwrap_or_default();
    let content = args.content.unwrap_or_default();
    if title.trim().is_empty() && content.trim().is_empty() {
        anyhow::bail!("标题和内容不能同时为空");
    }
    let input = AddMemoInput {
        title,
        content,
        visibility: args.visibility,
        category: args.category,
        tags: args.tags,
    };
    let m = svc.add_memo(&input)?;
    if json {
        println!("{}", serde_json::to_string(&m)?);
    } else {
        format::print_memo(&m, false);
        println!("\n✅ 已添加");
    }
    Ok(())
}

pub async fn run_memo_list(args: MemoListArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let filter = MemoFilter {
        category: args.category,
        tag: args.tag,
        favorited_only: args.pinned,
        limit: Some(args.limit),
        ..Default::default()
    };
    let list = svc.list_memos(&filter)?;
    format::print_memos(&list, json);
    Ok(())
}

pub async fn run_memo_get(args: MemoGetArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let m = svc.get_memo(&args.short_id)?;
    format::print_memo(&m, json);
    Ok(())
}

pub async fn run_memo_update(args: MemoUpdateArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let cur = svc.get_memo(&args.short_id)?;
    // 未提供的字段保留原值
    let tags = if args.tags.is_empty() {
        None
    } else {
        Some(args.tags)
    };
    let pinned = if args.pin {
        Some(true)
    } else if args.unpin {
        Some(false)
    } else {
        None
    };
    let category = if args.category.is_some() {
        Some(args.category)
    } else {
        None
    };
    let input = UpdateMemoInput {
        title: args.title.or(Some(cur.title)),
        content: args.content.or(Some(cur.content)),
        visibility: args.visibility.or(Some(cur.visibility)),
        category,
        tags,
        pinned,
    };
    let m = svc.update_memo(&args.short_id, &input)?;
    if json {
        println!("{}", serde_json::to_string(&m)?);
    } else {
        format::print_memo(&m, false);
        println!("\n✅ 已更新");
    }
    Ok(())
}

pub async fn run_memo_delete(args: MemoDeleteArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    if !args.yes && !json {
        print!("确认删除备忘录 {}？[y/N] ", args.short_id);
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        if !buf.trim().eq_ignore_ascii_case("y") {
            println!("已取消");
            return Ok(());
        }
    }
    svc.delete_memo(&args.short_id)?;
    if json {
        println!("{{\"deleted\":\"{}\"}}", args.short_id);
    } else {
        println!("🗑️  已移入回收站：{}", args.short_id);
    }
    Ok(())
}

pub async fn run_memo_search(args: MemoSearchArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let hits = svc.search_memos(&args.query, args.limit, args.offset)?;
    format::print_memo_search(&hits, json);
    Ok(())
}

pub async fn run_memo_tag(args: MemoTagArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let cur = svc.get_memo(&args.short_id)?;
    let mut tags: std::collections::BTreeSet<String> = cur.tags.iter().cloned().collect();
    for t in &args.add {
        tags.insert(t.clone());
    }
    for t in &args.remove {
        tags.remove(t);
    }
    let new_tags: Vec<String> = tags.into_iter().collect();
    let input = UpdateMemoInput {
        title: None,
        content: None,
        visibility: None,
        category: None,
        tags: Some(new_tags),
        pinned: None,
    };
    let m = svc.update_memo(&args.short_id, &input)?;
    if json {
        println!("{}", serde_json::to_string(&m)?);
    } else {
        format::print_memo(&m, false);
        println!("\n✅ 已更新标签");
    }
    Ok(())
}

pub async fn run_memo_pin(args: MemoPinArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let m = svc.toggle_memo_pinned(&args.short_id)?;
    if json {
        println!("{}", serde_json::to_string(&m)?);
    } else {
        let status = if m.pinned { "已置顶" } else { "已取消置顶" };
        println!("📌 {status}：{}", m.short_id);
    }
    Ok(())
}

pub async fn run_memo_visibility(args: MemoVisibilityArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let m = svc.set_memo_visibility(&args.short_id, &args.visibility)?;
    if json {
        println!("{}", serde_json::to_string(&m)?);
    } else {
        println!("👁️  可见性已设为 {}：{}", m.visibility, m.short_id);
    }
    Ok(())
}

pub async fn run_memo_link(args: MemoLinkArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    svc.create_reference(&args.source_id, &args.target_id)?;
    if json {
        println!("{}", serde_json::json!({"ok": true, "source": args.source_id, "target": args.target_id}));
    } else {
        println!("🔗 已创建引用：{} ↔ {}", args.source_id, args.target_id);
    }
    Ok(())
}

pub async fn run_memo_unlink(args: MemoUnlinkArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    svc.delete_reference(&args.source_id, &args.target_id)?;
    if json {
        println!("{}", serde_json::json!({"ok": true, "source": args.source_id, "target": args.target_id}));
    } else {
        println!("🔗 已删除引用：{} ↔ {}", args.source_id, args.target_id);
    }
    Ok(())
}

pub async fn run_memo_relations(args: MemoRelationsArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let relations = svc.get_all_relations(&args.short_id)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&relations)?);
    } else {
        if relations.is_empty() {
            println!("暂无关联关系");
            return Ok(());
        }
        println!("关联关系：");
        for rel in &relations {
            let arrow = if rel.relation_type == "COMMENT" { "💬" } else { "🔗" };
            println!("  {} {} ({})", arrow, rel.title, rel.short_id);
        }
    }
    Ok(())
}

pub async fn run_memo_history(args: MemoHistoryArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let versions = svc.list_versions(&args.short_id)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&versions)?);
    } else {
        if versions.is_empty() {
            println!("暂无版本历史");
            return Ok(());
        }
        println!("版本历史（共 {} 个版本）：", versions.len());
        for (i, v) in versions.iter().enumerate() {
            let content_preview: String = v.content.chars().take(50).collect();
            let suffix = if v.content.len() > 50 { "..." } else { "" };
            println!("  {}. [{}] {}{}", i + 1, v.id, content_preview, suffix);
            println!("     时间: {}", v.created_at);
        }
    }
    Ok(())
}

pub async fn run_memo_restore_version(args: MemoRestoreVersionArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let m = svc.restore_version(&args.short_id, args.version_id)?;
    if json {
        println!("{}", serde_json::to_string(&m)?);
    } else {
        println!("⏪ 已回滚到版本 {}：{}", args.version_id, m.short_id);
    }
    Ok(())
}
