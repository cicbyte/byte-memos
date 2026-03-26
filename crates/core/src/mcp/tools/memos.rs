//! 备忘录类工具：详情 / 列表 / 搜索（返回 Markdown，AI 友好）

use super::ToolCtx;

/// get_memo：按 shortId 获取备忘录详情
pub fn get_memo(ctx: &ToolCtx, short_id: &str) -> String {
    match ctx.svc.get_memo(short_id) {
        Ok(m) => {
            let mut md = format!("# {}\n", m.title);
            md.push_str(&format!("**ID**: `{}`\n\n", m.short_id));
            let mut meta: Vec<String> = Vec::new();
            if let Some(c) = &m.category {
                meta.push(format!("分类: {c}"));
            }
            meta.push(format!("创建: {}", &m.created_at[..10]));
            meta.push(format!("可见性: {}", m.visibility));
            if m.pinned {
                meta.push("📌 置顶".into());
            }
            md.push_str(&format!("**{}**\n\n", meta.join(" · ")));
            if !m.content.is_empty() {
                md.push_str(&format!("{}\n\n", m.content));
            }
            if !m.tags.is_empty() {
                md.push_str(&format!("**标签**: {}\n", m.tags.join(", ")));
            }
            md
        }
        Err(e) => format!("⚠️ 备忘录 `{short_id}` 不存在或读取失败：{e}"),
    }
}

/// memo_search：全文搜索备忘录
pub fn memo_search(ctx: &ToolCtx, query: &str, limit: i64, offset: i64) -> String {
    match ctx.svc.search_memos(query, limit.max(1), offset.max(0)) {
        Ok(hits) => {
            if hits.is_empty() {
                return format!("未找到匹配「{query}」的备忘录。");
            }
            let mut md = format!("找到 {} 个匹配「{query}」的备忘录：\n\n", hits.len());
            for h in hits {
                md.push_str(&format!("- **{}** `{}`\n", h.title, h.short_id));
            }
            md
        }
        Err(e) => format!("⚠️ 搜索失败：{e}"),
    }
}

/// memo_list：列出备忘录
pub fn memo_list(ctx: &ToolCtx, limit: i64, offset: i64) -> String {
    use crate::model::MemoFilter;
    let filter = MemoFilter {
        limit: Some(limit.clamp(1, 200)),
        offset: Some(offset.max(0)),
        ..Default::default()
    };
    match ctx.svc.list_memos(&filter) {
        Ok(list) => {
            if list.is_empty() {
                return "（暂无备忘录）".into();
            }
            let mut md = String::new();
            for m in &list {
                md.push_str(&format!("- `{}` {}\n", m.short_id, m.title));
            }
            md.push_str(&format!("\n共 {} 个备忘录。", list.len()));
            md
        }
        Err(e) => format!("⚠️ 读取失败：{e}"),
    }
}
