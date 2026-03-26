//! 备忘录输出：详情 / 列表

use byte_memos_core::model::{Memo, MemoSearchHit};

use super::{paint, truncate, pad, BOLD, GRAY, CYAN, BLUE, GREEN, YELLOW, RESET};

pub fn print_memo(m: &Memo, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(m).unwrap());
        return;
    }
    println!("{}", paint(BOLD, &m.title));
    println!("  {} {}", paint(GRAY, "id:"), paint(CYAN, &m.short_id));
    if !m.content.is_empty() {
        let preview = truncate(&m.content, 80);
        println!("  {} {}", paint(GRAY, "content:"), preview);
    }
    println!("  {} {}", paint(GRAY, "visibility:"), paint_visibility(&m.visibility));
    if m.pinned {
        println!("  {} 📌", paint(GRAY, "pinned:"));
    }
    if let Some(c) = &m.category {
        println!("  {} {}", paint(GRAY, "category:"), c);
    }
    if !m.tags.is_empty() {
        let tags = m
            .tags
            .iter()
            .map(|t| paint(BLUE, &format!("#{t}")))
            .collect::<Vec<_>>()
            .join(" ");
        println!("  {} {tags}", paint(GRAY, "tags:"));
    }
    println!(
        "  {} {} · {} {}",
        paint(GRAY, "created:"),
        m.created_at,
        paint(GRAY, "updated:"),
        m.updated_at,
    );
}

fn paint_visibility(v: &str) -> String {
    match v {
        "PUBLIC" => paint(GREEN, v),
        "PROTECTED" => paint(YELLOW, v),
        _ => paint(GRAY, v),
    }
}

pub fn print_memos(list: &[Memo], json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(list).unwrap());
        return;
    }
    if list.is_empty() {
        println!("{}", paint(GRAY, "（无备忘录）"));
        return;
    }
    for m in list {
        let title = truncate(&m.title, 32);
        let pin = if m.pinned { "📌 " } else { "   " };
        let vis = match m.visibility.as_str() {
            "PUBLIC" => paint(GREEN, "P"),
            "PROTECTED" => paint(YELLOW, "R"),
            _ => paint(GRAY, "-"),
        };
        println!(
            "{}{} {} {}  {}",
            pin,
            paint(CYAN, &format!("{:<10}", m.short_id)),
            vis,
            pad(&title, 32),
            paint(GRAY, &m.created_at),
        );
    }
    println!("{}", paint(GRAY, &format!("共 {} 个备忘录", list.len())));
}

pub fn print_memo_search(hits: &[MemoSearchHit], json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(hits).unwrap());
        return;
    }
    if hits.is_empty() {
        println!("{}", paint(GRAY, "（无匹配）"));
        return;
    }
    for h in hits {
        println!(
            "{} {}",
            paint(CYAN, &h.short_id),
            paint(BOLD, &h.title),
        );
        if let Some(snip) = &h.snippet {
            if !snip.is_empty() {
                let pretty = snip
                    .replace("<mark>", &format!("{YELLOW}{BOLD}"))
                    .replace("</mark>", RESET);
                println!("    {pretty}");
            }
        }
    }
    println!("{}", paint(GRAY, &format!("共 {} 项命中", hits.len())));
}
