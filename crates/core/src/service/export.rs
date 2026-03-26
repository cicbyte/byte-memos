//! 笔记导出服务（HTML / Markdown）。

use super::*;

impl super::ProjectService {
    /// 导出笔记为 HTML
    pub fn export_memo_html(&self, short_id: &str) -> CoreResult<String> {
        let memo = self.get_memo(short_id)?;
        let html_content = markdown_to_html(&memo.content);
        
        let tags_str = if memo.tags.is_empty() {
            String::new()
        } else {
            let tags: Vec<String> = memo.tags.iter().map(|t| format!("<span class=\"tag\">{}</span>", t)).collect();
            format!("<div class=\"tags\">标签：{}</div>", tags.join(" "))
        };

        let category_str = match &memo.category {
            Some(c) => format!("<div class=\"category\">分类：{}</div>", c),
            None => String::new(),
        };

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; line-height: 1.6; color: #333; }}
        h1 {{ border-bottom: 2px solid #eee; padding-bottom: 10px; }}
        pre {{ background: #f5f5f5; padding: 12px; border-radius: 4px; overflow-x: auto; }}
        code {{ background: #f0f0f0; padding: 2px 4px; border-radius: 3px; }}
        pre code {{ background: none; padding: 0; }}
        blockquote {{ border-left: 4px solid #ddd; margin: 0; padding-left: 16px; color: #666; }}
        img {{ max-width: 100%; }}
        .meta {{ color: #888; font-size: 0.9em; margin-bottom: 20px; }}
        .tag {{ background: #e0e0e0; padding: 2px 8px; border-radius: 12px; font-size: 0.85em; margin-right: 4px; }}
        .tags, .category {{ margin: 4px 0; color: #666; }}
        @media print {{ body {{ max-width: none; }} }}
    </style>
</head>
<body>
    <h1>{title}</h1>
    <div class="meta">
        <div>创建时间：{created_at}</div>
        <div>更新时间：{updated_at}</div>
        {category}
        {tags}
    </div>
    <hr>
    {content}
</body>
</html>"#,
            title = html_escape(&memo.title),
            created_at = memo.created_at,
            updated_at = memo.updated_at,
            category = category_str,
            tags = tags_str,
            content = html_content,
        );

        Ok(html)
    }

    /// 导出笔记为 Markdown（原始内容）
    pub fn export_memo_markdown(&self, short_id: &str) -> CoreResult<String> {
        let memo = self.get_memo(short_id)?;
        Ok(memo.content)
    }
}

/// 简单的 Markdown 转 HTML
fn markdown_to_html(md: &str) -> String {
    let mut html = String::new();
    let mut in_code_block = false;
    let mut code_lang;

    for line in md.lines() {
        if line.starts_with("```") {
            if in_code_block {
                html.push_str("</code></pre>\n");
                in_code_block = false;
            } else {
                code_lang = line[3..].trim().to_string();
                if code_lang.is_empty() {
                    html.push_str("<pre><code>");
                } else {
                    html.push_str(&format!("<pre><code class=\"language-{}\">", html_escape(&code_lang)));
                }
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            html.push_str(&html_escape(line));
            html.push('\n');
            continue;
        }

        // 标题
        if line.starts_with("# ") {
            html.push_str(&format!("<h1>{}</h1>\n", html_escape(&line[2..])));
        } else if line.starts_with("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", html_escape(&line[3..])));
        } else if line.starts_with("### ") {
            html.push_str(&format!("<h3>{}</h3>\n", html_escape(&line[4..])));
        } else if line.starts_with("#### ") {
            html.push_str(&format!("<h4>{}</h4>\n", html_escape(&line[5..])));
        } else if line.starts_with("- ") || line.starts_with("* ") {
            html.push_str(&format!("<li>{}</li>\n", html_escape(&line[2..])));
        } else if line.starts_with("> ") {
            html.push_str(&format!("<blockquote>{}</blockquote>\n", html_escape(&line[2..])));
        } else if line.trim().is_empty() {
            html.push('\n');
        } else {
            // 处理行内格式
            let mut processed = html_escape(line);
            // 粗体
            processed = replace_pattern(&processed, "**", "<strong>", "</strong>");
            // 斜体
            processed = replace_pattern(&processed, "*", "<em>", "</em>");
            // 行内代码
            processed = replace_inline_code(&processed);
            html.push_str(&format!("<p>{}</p>\n", processed));
        }
    }

    if in_code_block {
        html.push_str("</code></pre>\n");
    }

    html
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn replace_pattern(s: &str, marker: &str, open: &str, close: &str) -> String {
    let parts: Vec<&str> = s.split(marker).collect();
    if parts.len() >= 3 {
        let mut result = String::new();
        for i in 0..parts.len() {
            if i > 0 {
                if i % 2 == 1 {
                    result.push_str(open);
                } else {
                    result.push_str(close);
                }
            }
            result.push_str(parts[i]);
        }
        result
    } else {
        s.to_string()
    }
}

fn replace_inline_code(s: &str) -> String {
    let mut result = String::new();
    let mut in_code = false;
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '`' {
            if in_code {
                result.push_str("</code>");
            } else {
                result.push_str("<code>");
            }
            in_code = !in_code;
        } else {
            result.push(c);
        }
    }

    if in_code {
        result.push_str("</code>");
    }

    result
}
