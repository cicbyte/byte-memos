//! AI 动作提示词模板（备忘录场景）
//!
//! memos 不是长文写作：动作围绕「记完之后」——把随手记整理成型、摘要、
//! 续写/扩写、润色、翻译，以及 JSON 结构输出（标签/分类建议、待办提取）。
//! 所有模板要求「直接返回结果，不加前缀/解释」，方便直接落库。

/// 可用动作（与前端 AI 菜单一一对应）
pub const ACTIONS: &[&str] = &[
    "format",
    "summary",
    "continue",
    "expand",
    "polish",
    "translate",
    "tags",
    "category",
    "todos",
    "check",
];

/// 动作是否输出 JSON（决定前端能否做 chip 采纳等结构化处理）
pub fn is_json_action(action: &str) -> bool {
    matches!(action, "tags" | "category")
}

/// 组装动作提示词
///
/// - `action`：见 [`ACTIONS`]，未知回落 format
/// - `content`：备忘录正文
/// - `title`：备忘录标题（可为空）
/// - `lang`：目标语言（translate 用，如「英文」）
pub fn get_prompt(action: &str, content: &str, title: &str, lang: &str) -> String {
    let title_hint = if title.trim().is_empty() {
        String::new()
    } else {
        format!("标题：{}\n\n", title.trim())
    };
    let body = |instruction: &str| {
        format!(
            "{title_hint}以下是备忘录内容：\n\n{content}\n\n{instruction}"
        )
    };
    match action {
        "summary" => body(
            "请为这篇备忘录生成一段简洁摘要（TL;DR），保留核心要点，控制在 120 字以内。\
             直接返回摘要文本，不要任何前缀或解释。",
        ),
        "continue" => body(
            "请顺着这篇备忘录的思路和语气自然续写下去，补充合理的展开内容。\
             直接返回续写部分（不要重复原文），不要任何前缀或解释。",
        ),
        "expand" => body(
            "请把这篇备忘录扩写成更完整的版本：补充细节、例子和结构，\
             保留原始意图与语气。直接返回扩写后的全文，不要任何前缀或解释。",
        ),
        "polish" => body(
            "请润色这篇备忘录：修正错别字与语病，让表达更流畅准确，\
             不改变原意与篇幅量级。直接返回润色后的全文，不要任何前缀或解释。",
        ),
        "translate" => body(&format!(
            "请将这篇备忘录完整翻译为{lang}，保持 Markdown 格式与原文结构。\
             直接返回译文，不要任何前缀或解释。"
        )),
        "format" => body(
            "请把这篇随手记整理成结构清晰的 Markdown 笔记：\
             提炼一个简短标题行（# 开头），正文用列表/小标题组织要点，\
             修正口语化和错别字，不遗漏原始信息，不编造新内容。\
             直接返回整理后的 Markdown，不要任何前缀或解释。",
        ),
        "tags" => body(
            "请为这篇备忘录推荐标签：输出一个 JSON 字符串数组（最多 5 个），\
             标签用简短中文词，体现主题与用途。只返回 JSON 数组本身，不要解释。\
             示例：[\"工作\", \"方案评审\"]",
        ),
        "category" => {
            let cats = if lang.trim().is_empty() {
                String::new()
            } else {
                format!("候选分类：{}\n", lang.trim())
            };
            format!(
                "{title_hint}以下是备忘录内容：\n\n{content}\n\n{cats}\
                 请从候选分类中选出最匹配的一个分类名（必须原样使用候选名）；\
                 若都不匹配或无候选，返回空字符串。\
                 只返回 JSON：{{\"category\": \"...\"}}，不要解释。"
            )
        }
        "todos" => body(
            "请从这篇备忘录中提取所有待办事项（包括隐含的行动项），\
             每条一行，使用 Markdown 任务列表语法「- [ ] 」，动词开头、具体可执行。\
             直接返回任务列表，不要任何前缀或解释。",
        ),
        "check" => "请回复「连接正常」四个字，不要输出任何其他内容。".to_string(),
        _ => get_prompt("format", content, title, lang),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_covered_and_fallback() {
        for a in ACTIONS {
            let p = get_prompt(a, "内容", "标题", "英文");
            assert!(!p.is_empty());
        }
        // 未知动作回落 format
        assert_eq!(
            get_prompt("nope", "c", "", ""),
            get_prompt("format", "c", "", "")
        );
    }

    #[test]
    fn json_actions_declared() {
        assert!(is_json_action("tags"));
        assert!(is_json_action("category"));
        assert!(!is_json_action("polish"));
    }

    #[test]
    fn translate_carries_lang() {
        assert!(get_prompt("translate", "hi", "", "英文").contains("英文"));
    }
}
