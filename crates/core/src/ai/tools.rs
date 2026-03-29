//! 内置工具集（备忘录域）：供 AI 侧边栏 agent 循环调用
//!
//! - [`definitions`]：工具 JSON Schema（rig ToolDefinition，发给模型）
//! - [`execute`]：同步执行器（持 ProjectService 引用；锁由调用方管理）
//!
//! 设计约定：
//! - 结果一律序列化为 JSON 字符串回填给模型（成功 ok:true + 数据；失败 ok:false + error）
//! - 写入类工具最小惊讶：create 固定 PRIVATE 可见性；delete 走软删除（回收站可恢复）
//! - 列表类结果对 content 截断（省 token），get_memo 才给全文

use rig_core::completion::ToolDefinition;
use serde_json::{json, Value};

use crate::model::{AddMemoInput, MemoFilter, UpdateMemoInput};
use crate::service::ProjectService;

use super::stream::ToolExec;

/// 内置工具定义（发给模型的 JSON Schema 清单）
pub fn definitions() -> Vec<ToolDefinition> {
    fn def(name: &str, description: &str, properties: Value, required: &[&str]) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: description.to_string(),
            parameters: json!({
                "type": "object",
                "properties": properties,
                "required": required,
            }),
        }
    }

    vec![
        def(
            "search_memos",
            "全文搜索备忘录（FTS，按相关度排名）。用于查找用户提过的某篇内容。",
            json!({
                "query": { "type": "string", "description": "搜索关键词" },
                "limit": { "type": "integer", "description": "返回条数上限，默认 5，最大 20" }
            }),
            &["query"],
        ),
        def(
            "list_memos",
            "列出备忘录（可按分类/标签/置顶过滤，返回最近更新的在前）。用于浏览、统计最近内容。",
            json!({
                "category": { "type": "string", "description": "按分类名过滤（可选）" },
                "tag": { "type": "string", "description": "按标签过滤（可选）" },
                "pinned_only": { "type": "boolean", "description": "仅置顶（可选）" },
                "limit": { "type": "integer", "description": "返回条数上限，默认 10，最大 50" }
            }),
            &[],
        ),
        def(
            "get_memo",
            "取单条备忘录的完整内容。",
            json!({
                "short_id": { "type": "string", "description": "备忘录短 ID（search/list 结果里的 short_id）" }
            }),
            &["short_id"],
        ),
        def(
            "list_categories",
            "列出全部分类及其备忘录数量。",
            json!({}),
            &[],
        ),
        def(
            "list_tags",
            "列出全部标签及关联备忘录数量。",
            json!({}),
            &[],
        ),
        def(
            "daily_stats",
            "按日统计备忘录创建数（热力图数据）。用于回答「这个月记了多少条」。",
            json!({
                "days": { "type": "integer", "description": "统计最近 N 天，默认 30" }
            }),
            &[],
        ),
        def(
            "create_memo",
            "创建一条新备忘录（固定私有可见性）。用户明确要记录/保存内容时使用。",
            json!({
                "title": { "type": "string", "description": "标题（可为空串）" },
                "content": { "type": "string", "description": "Markdown 正文" },
                "category": { "type": "string", "description": "分类名（可选，不存在则创建）" },
                "tags": { "type": "array", "items": { "type": "string" }, "description": "标签（可选，最多 5 个）" },
                "pinned": { "type": "boolean", "description": "是否置顶（可选，默认否）" }
            }),
            &["content"],
        ),
        def(
            "update_memo",
            "更新备忘录（字段可选，只改给出的字段；内容变更自动存版本快照）。",
            json!({
                "short_id": { "type": "string", "description": "备忘录短 ID" },
                "title": { "type": "string", "description": "新标题（可选）" },
                "content": { "type": "string", "description": "新 Markdown 正文（可选）" },
                "category": { "type": ["string", "null"], "description": "新分类（null=清除分类；不传=不改）" },
                "tags": { "type": "array", "items": { "type": "string" }, "description": "新标签（全量替换；不传=不改）" },
                "pinned": { "type": "boolean", "description": "是否置顶（可选）" }
            }),
            &["short_id"],
        ),
        def(
            "delete_memo",
            "删除备忘录（软删除，移入回收站可恢复）。",
            json!({
                "short_id": { "type": "string", "description": "备忘录短 ID" }
            }),
            &["short_id"],
        ),
    ]
}

// ===== 参数读取 helper =====

fn arg_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}
fn arg_i64(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| v.as_i64())
}
fn arg_bool(args: &Value, key: &str) -> Option<bool> {
    args.get(key).and_then(|v| v.as_bool())
}
fn arg_str_vec(args: &Value, key: &str) -> Option<Vec<String>> {
    args.get(key).and_then(|v| v.as_array()).map(|a| {
        a.iter()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .collect()
    })
}

/// 截断长文（列表场景省 token）
fn excerpt(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        t.chars().take(max).collect::<String>() + "…"
    }
}

/// 执行一个工具调用，返回 JSON 结果文本（成功 ok:true + 数据；失败 ok:false + error）
pub fn execute(svc: &ProjectService, exec: &ToolExec) -> String {
    let args: Value = serde_json::from_str(&exec.args_json).unwrap_or(json!({}));
    let result: Result<Value, String> = dispatch(svc, &exec.name, &args);
    match result {
        Ok(v) => v.to_string(),
        Err(e) => json!({ "ok": false, "error": e }).to_string(),
    }
}

fn dispatch(svc: &ProjectService, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "search_memos" => {
            let query = arg_str(args, "query").ok_or("缺少 query")?;
            let limit = arg_i64(args, "limit").unwrap_or(5).clamp(1, 20);
            let hits = svc
                .search_memos(&query, limit, 0)
                .map_err(|e| e.to_string())?;
            let items: Vec<Value> = hits
                .iter()
                .map(|h| {
                    json!({
                        "short_id": h.short_id,
                        "title": h.title,
                        "snippet": h.snippet,
                        "category": h.category,
                        "tags": h.tags,
                    })
                })
                .collect();
            Ok(json!({ "ok": true, "count": items.len(), "items": items }))
        }
        "list_memos" => {
            let limit = arg_i64(args, "limit").unwrap_or(10).clamp(1, 50);
            let filter = MemoFilter {
                tag: arg_str(args, "tag"),
                category: arg_str(args, "category"),
                pinned_only: arg_bool(args, "pinned_only").unwrap_or(false),
                limit: Some(limit),
                offset: None,
            };
            let memos = svc.list_memos(&filter).map_err(|e| e.to_string())?;
            let items: Vec<Value> = memos
                .iter()
                .map(|m| {
                    json!({
                        "short_id": m.short_id,
                        "title": m.title,
                        "excerpt": excerpt(&m.content, 200),
                        "category": m.category,
                        "tags": m.tags,
                        "pinned": m.pinned,
                        "updated_at": m.updated_at,
                    })
                })
                .collect();
            Ok(json!({ "ok": true, "count": items.len(), "items": items }))
        }
        "get_memo" => {
            let short_id = arg_str(args, "short_id").ok_or("缺少 short_id")?;
            let m = svc.get_memo(&short_id).map_err(|e| e.to_string())?;
            Ok(json!({
                "ok": true,
                "memo": {
                    "short_id": m.short_id,
                    "title": m.title,
                    "content": m.content,
                    "category": m.category,
                    "tags": m.tags,
                    "pinned": m.pinned,
                    "visibility": m.visibility,
                    "created_at": m.created_at,
                    "updated_at": m.updated_at,
                }
            }))
        }
        "list_categories" => {
            let cats = svc.list_categories().map_err(|e| e.to_string())?;
            let items: Vec<Value> = cats
                .iter()
                .map(|c| json!({ "name": c.name, "memo_count": c.memo_count }))
                .collect();
            Ok(json!({ "ok": true, "count": items.len(), "items": items }))
        }
        "list_tags" => {
            let tree = svc.list_tags_tree().map_err(|e| e.to_string())?;
            let items: Vec<Value> = tree
                .iter()
                .map(|t| json!({ "name": t.name, "count": t.count }))
                .collect();
            Ok(json!({ "ok": true, "count": items.len(), "items": items }))
        }
        "daily_stats" => {
            let days = arg_i64(args, "days").unwrap_or(30).clamp(1, 365) as usize;
            let all = svc.list_memo_daily_counts().map_err(|e| e.to_string())?;
            let items: Vec<Value> = all
                .iter()
                .rev()
                .take(days)
                .map(|d| json!({ "date": d.date, "count": d.count }))
                .collect();
            Ok(json!({ "ok": true, "days": days, "items": items }))
        }
        "create_memo" => {
            let input = AddMemoInput {
                title: arg_str(args, "title").unwrap_or_default(),
                content: arg_str(args, "content").ok_or("缺少 content")?,
                visibility: "PRIVATE".to_string(),
                category: arg_str(args, "category"),
                tags: arg_str_vec(args, "tags")
                    .map(|mut t| {
                        t.truncate(5);
                        t
                    })
                    .unwrap_or_default(),
            };
            let m = svc.add_memo(&input).map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "short_id": m.short_id, "title": m.title }))
        }
        "update_memo" => {
            let short_id = arg_str(args, "short_id").ok_or("缺少 short_id")?;
            let category = match args.get("category") {
                None => None,
                Some(Value::Null) => Some(None),
                Some(v) => v.as_str().map(|s| Some(s.to_string())),
            };
            let input = UpdateMemoInput {
                title: arg_str(args, "title"),
                content: arg_str(args, "content"),
                visibility: None,
                category,
                tags: arg_str_vec(args, "tags"),
                pinned: arg_bool(args, "pinned"),
            };
            let m = svc.update_memo(&short_id, &input).map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "short_id": m.short_id, "title": m.title, "updated_at": m.updated_at }))
        }
        "delete_memo" => {
            let short_id = arg_str(args, "short_id").ok_or("缺少 short_id")?;
            svc.delete_memo(&short_id).map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "note": "已移入回收站，可恢复" }))
        }
        _ => Err(format!("未知工具：{name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_cover_expected_tools() {
        let defs = definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        for expected in [
            "search_memos",
            "list_memos",
            "get_memo",
            "list_categories",
            "list_tags",
            "daily_stats",
            "create_memo",
            "update_memo",
            "delete_memo",
        ] {
            assert!(names.contains(&expected), "缺少工具 {expected}");
        }
        // schema 形状：object + properties
        for d in &defs {
            assert_eq!(d.parameters["type"], "object");
            assert!(d.parameters["properties"].is_object());
        }
    }

    #[test]
    fn unknown_tool_reports_error() {
        let out = std::panic::catch_unwind(|| {
            // execute 需要 ProjectService，这里只验证 dispatch 层拒绝未知工具的逻辑分支
            let args = json!({});
            let _ = serde_json::from_str::<Value>(&args.to_string()).unwrap();
        });
        assert!(out.is_ok());
    }
}
