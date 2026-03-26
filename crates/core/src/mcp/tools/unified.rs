//! 统一入口工具：search/list/get/delete + 统计/标签/分类
//!
//! 各 `unified_*` 按 `type` 分发；知识库类型已移除，仅剩 memo。
//! search_all / stats / list_categories 在 core 退化为备忘录单一类型后，
//! 返回值与展示均仅含备忘录维度。

use super::memos::{memo_list, memo_search, get_memo};
use super::helpers::{format_size, strip_marks};
use super::ToolCtx;

/// search_all：统一搜索（core 退化为备忘录后仅返回备忘录命中）
pub fn search_all(ctx: &ToolCtx, query: &str, limit: i64, offset: i64) -> String {
    match ctx.svc.search_all(query, limit.max(1).min(50), offset.max(0)) {
        Ok(hits) => {
            if hits.is_empty() {
                return format!("未找到匹配「{query}」的内容。");
            }
            let mut md = format!("找到 {} 项匹配「{query}」：\n\n", hits.len());
            for h in hits {
                // ItemType 退化为仅 Memo，统一展示为「备忘录」
                let type_label = "备忘录";
                let snip = strip_marks(&h.snippet);
                if snip.is_empty() {
                    md.push_str(&format!("- [{}] **{}** `{}`\n", type_label, h.title, h.short_id));
                } else {
                    md.push_str(&format!("- [{}] **{}** `{}` — {}\n", type_label, h.title, h.short_id, snip));
                }
            }
            md
        }
        Err(e) => format!("⚠️ 搜索失败：{e}"),
    }
}

/// stats：返回统计概览（备忘录维度 + 共享的标签/分类/存储）
pub fn stats(ctx: &ToolCtx) -> String {
    match ctx.svc.stats_overview() {
        Ok(s) => {
            let mut md = String::from("# 概览\n\n");
            md.push_str(&format!("- **备忘录**: {}\n", s.memo_count));
            md.push_str(&format!("- **标签**: {}\n", s.tag_count));
            md.push_str(&format!("- **分类**: {}\n", s.category_count));
            md.push_str(&format!("- **存储占用**: {}\n", format_size(s.total_size)));
            md.push_str(&format!("- **置顶**: {}\n", s.pinned_count));
            md.push_str(&format!("- **回收站**: {}\n", s.trashed_count));
            md
        }
        Err(e) => format!("⚠️ 统计失败：{e}"),
    }
}

/// list_tags：列出全部标签及使用次数
pub fn list_tags(ctx: &ToolCtx, limit: i64) -> String {
    match ctx.svc.tag_stats(limit.max(1).min(500)) {
        Ok(tags) => {
            if tags.is_empty() {
                return "（暂无标签）".into();
            }
            let mut md = String::new();
            for t in tags {
                md.push_str(&format!("- `{}` ({})\n", t.tag, t.count));
            }
            md
        }
        Err(e) => format!("⚠️ 读取失败：{e}"),
    }
}

/// list_categories：列出全部分类（每分类下的备忘录数）
pub fn list_categories(ctx: &ToolCtx) -> String {
    match ctx.svc.list_categories() {
        Ok(cats) => {
            if cats.is_empty() {
                return "（暂无分类）".into();
            }
            let mut md = String::new();
            for c in cats {
                md.push_str(&format!("- {} (备忘录 {})\n", c.name, c.memo_count));
            }
            md
        }
        Err(e) => format!("⚠️ 读取失败：{e}"),
    }
}

// ============ 统一工具（合并入口）============

/// search：统一搜索（type 控制搜哪类，all=跨类型）
pub fn unified_search(ctx: &ToolCtx, query: &str, item_type: &str, limit: i64, offset: i64) -> String {
    match item_type {
        "memo" => memo_search(ctx, query, limit, offset),
        _ => search_all(ctx, query, limit, offset), // "all" 或其他值
    }
}

/// list：统一列表（type 控制列哪类）
pub fn unified_list(ctx: &ToolCtx, item_type: &str, limit: i64, offset: i64) -> String {
    match item_type {
        "memo" => memo_list(ctx, limit, offset),
        "tag" => list_tags(ctx, limit),
        "category" => list_categories(ctx),
        _ => "⚠️ type 必须是 memo / tag / category 之一".into(),
    }
}

/// get：统一读取（type 控制读哪类）
pub fn unified_get(ctx: &ToolCtx, short_id: &str, item_type: &str) -> String {
    match item_type {
        "memo" => get_memo(ctx, short_id),
        _ => "⚠️ type 必须是 memo".into(),
    }
}

/// delete：统一删除（仅移入回收站，AI 不提供彻底删除以防误操作）
pub fn unified_delete(ctx: &ToolCtx, short_id: &str, item_type: &str) -> String {
    match item_type {
        "memo" => match ctx.svc.delete_memo(short_id) {
            Ok(()) => format!("✅ 已移入回收站：备忘录 `{}`", short_id),
            Err(e) => format!("⚠️ 删除失败：{e}"),
        },
        _ => "⚠️ type 必须是 memo".into(),
    }
}
