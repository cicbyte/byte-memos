//! AI 命令（byte-memos AI 域，feature 由 core ai 提供）
//!
//! - 供应商 CRUD：薄封装 ProjectService（api_key 永不回显）
//! - ai_stream：流式动作补全。锁内取配置与拼 prompt，锁外驱动 rig 异步流，
//!   增量经 Channel 推前端；取消走 CancellationToken 注册表（token 边界中断）。
//!
//! 注意：ProjectService 用 std Mutex——不得在持锁时 .await。

use std::collections::HashMap;

use tauri::ipc::Channel;
use tauri::State;
use tokio_util::sync::CancellationToken;

use byte_memos_core::ai::prompts;
use byte_memos_core::ai::{StreamChunk, ProviderConfig};
use byte_memos_core::service::{
    AddProviderInput, CheckResult, ProviderItem, UpdateProviderInput,
};

use crate::state::AppState;

// ===== 供应商管理 =====

#[tauri::command]
pub fn ai_provider_list(state: State<'_, AppState>) -> Result<Vec<ProviderItem>, String> {
    state.svc().ai_provider_list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ai_provider_add(state: State<'_, AppState>, input: AddProviderInput) -> Result<i64, String> {
    state.svc().ai_provider_add(&input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ai_provider_update(
    state: State<'_, AppState>,
    input: UpdateProviderInput,
) -> Result<(), String> {
    state.svc().ai_provider_update(&input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ai_provider_delete(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.svc().ai_provider_delete(id).map_err(|e| e.to_string())
}

/// 连通测试：发一条最小 prompt 验证配置可用（异步 HTTP，tokio 直接驱动）
#[tauri::command]
pub async fn ai_provider_check(
    state: State<'_, AppState>,
    id: i64,
) -> Result<CheckResult, String> {
    let cfg = {
        let svc = state.svc();
        svc.ai_provider_config_by_id(id).map_err(|e| e.to_string())?
    };
    let prompt = prompts::get_prompt("check", "", "", "");
    match byte_memos_core::ai::stream::run_once(&cfg, "", &prompt).await {
        Ok(_) => Ok(CheckResult { ok: true, message: "连接成功".into() }),
        Err(e) => Ok(CheckResult { ok: false, message: format!("连接失败: {e}") }),
    }
}

/// 拉取供应商可用模型列表（GET /models 系端点；不落库，前端勾选后经 update 保存）
#[tauri::command]
pub async fn ai_provider_models(state: State<'_, AppState>, id: i64) -> Result<Vec<String>, String> {
    let cfg = {
        let svc = state.svc();
        svc.ai_provider_config_by_id(id).map_err(|e| e.to_string())?
    };
    byte_memos_core::ai::models::fetch_model_list(&cfg)
        .await
        .map_err(|e| e.to_string())
}

/// 前端可用动作清单（驱动 AI 菜单；is_json 标记结构化输出动作）
#[derive(serde::Serialize)]
pub struct AiActionInfo {
    pub action: String,
    pub is_json: bool,
}

#[tauri::command]
pub fn ai_actions() -> Vec<AiActionInfo> {
    prompts::ACTIONS
        .iter()
        .map(|a| AiActionInfo {
            action: a.to_string(),
            is_json: prompts::is_json_action(a),
        })
        .collect()
}

// ===== 流式补全 =====

/// 注册取消令牌（requestId → CancellationToken）
pub fn register_cancel(state: &AppState, request_id: &str) -> CancellationToken {
    let token = CancellationToken::new();
    state
        .ai_cancels
        .lock()
        .expect("ai_cancels poisoned")
        .insert(request_id.to_string(), token.clone());
    token
}

#[tauri::command]
pub fn ai_stream_cancel(state: State<'_, AppState>, request_id: String) -> Result<(), String> {
    if let Some(token) = state
        .ai_cancels
        .lock()
        .expect("ai_cancels poisoned")
        .remove(&request_id)
    {
        token.cancel();
    }
    Ok(())
}

/// 流式动作补全。返回聚合全文（取消时返回 Err(已取消)）。
///
/// 前端参数：action / content / title / lang / requestId + Channel。
#[tauri::command]
pub async fn ai_stream(
    state: State<'_, AppState>,
    action: String,
    content: String,
    title: String,
    lang: String,
    request_id: String,
    on_chunk: Channel<StreamChunk>,
) -> Result<String, String> {
    // 锁内：取启用供应商配置 + 拼 prompt（含分类候选注入）
    struct Prepared {
        cfg: ProviderConfig,
        prompt: String,
        system: String,
    }
    let prepared: Prepared = {
        let svc = state.svc();
        let cfg = svc
            .enabled_ai_provider_config()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未配置启用的 AI 供应商，请先在设置中配置".to_string())?;
        // category 动作：lang 参数位注入现有分类名清单
        let effective_lang = if prompts::is_json_action(&action) && action == "category" {
            svc.list_categories()
                .map(|cs| {
                    cs.into_iter()
                        .map(|c| c.name)
                        .collect::<Vec<_>>()
                        .join("、")
                })
                .unwrap_or_default()
        } else {
            lang
        };
        let prompt = prompts::get_prompt(&action, &content, &title, &effective_lang);
        drop(svc);
        Prepared { cfg, prompt, system: String::new() }
    };

    let token = register_cancel(&state, &request_id);
    let chan = on_chunk.clone();
    let result = byte_memos_core::ai::stream::run_stream(
        &prepared.cfg,
        &prepared.system,
        &prepared.prompt,
        &mut |token_delta: &str, thinking: &str| {
            if !thinking.is_empty() {
                let _ = chan.send(StreamChunk::Thinking { content: thinking.to_string() });
            }
            if !token_delta.is_empty() {
                let _ = chan.send(StreamChunk::Token { content: token_delta.to_string() });
            }
        },
        &token,
    )
    .await;

    state
        .ai_cancels
        .lock()
        .expect("ai_cancels poisoned")
        .remove(&request_id);

    match result {
        Ok(full) => {
            let _ = on_chunk.send(StreamChunk::Done { full: full.clone() });
            Ok(full)
        }
        Err(e) => {
            let msg = e.to_string();
            if msg == byte_memos_core::ai::stream::CANCELLED_MSG {
                // 取消：不发 Error，静默收尾；promise 返回 Err(已取消) 由前端识别
                Err(byte_memos_core::ai::stream::CANCELLED_MSG.to_string())
            } else {
                let _ = on_chunk.send(StreamChunk::Error { message: msg.clone() });
                Err(msg)
            }
        }
    }
}

/// AI 侧边栏多轮对话：messages 为完整会话（最后一条须为用户消息），
/// 其余按序作为历史注入请求。返回聚合全文。
#[derive(serde::Deserialize)]
pub struct ChatMsg {
    pub role: String,
    pub content: String,
}

#[tauri::command]
pub async fn ai_chat_stream(
    state: State<'_, AppState>,
    messages: Vec<ChatMsg>,
    request_id: String,
    on_chunk: Channel<StreamChunk>,
) -> Result<String, String> {
    let (cfg, history, user) = {
        let svc = state.svc();
        let cfg = svc
            .enabled_ai_provider_config()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "未配置启用的 AI 供应商，请先在设置中配置".to_string())?;
        let mut turns: Vec<(bool, String)> = messages
            .iter()
            .filter(|m| !m.content.trim().is_empty())
            .map(|m| (m.role != "assistant", m.content.clone()))
            .collect();
        // 容错：丢弃开头的助手消息（正常序列不会出现）
        while turns.first().map(|(is_user, _)| !*is_user).unwrap_or(false) {
            turns.remove(0);
        }
        let user = match turns.pop() {
            Some((true, c)) => c,
            _ => return Err("会话最后一条必须是用户消息".to_string()),
        };
        drop(svc);
        (cfg, turns, user)
    };
    let system = "你是 byte-memos 备忘录应用内置的 AI 助手。用简体中文简洁回答，\
                  可使用 Markdown 排版。你有内置工具可以查询与操作用户的备忘录\
                  （搜索、列出、读取、创建、更新、删除、分类/标签/统计），\
                  涉及用户数据的问题优先调用工具获取真实数据，不要编造；\
                  用户明确要求记录/保存时用 create_memo。";

    let token = register_cancel(&state, &request_id);
    let chan = on_chunk.clone();
    // 工具执行：锁内同步调用 ProjectService（不跨 await 持锁）
    let svc_state = state.clone();
    let execute = move |exec: &byte_memos_core::ai::stream::ToolExec| {
        let svc = svc_state.svc();
        byte_memos_core::ai::tools::execute(&svc, exec)
    };
    let result = byte_memos_core::ai::stream::run_agent_stream(
        &cfg,
        system,
        &history,
        &user,
        byte_memos_core::ai::tools::definitions(),
        execute,
        &mut |token_delta: &str, thinking: &str, tool: Option<&byte_memos_core::ai::stream::ToolExec>| {
            if let Some(t) = tool {
                let _ = chan.send(StreamChunk::Tool { name: t.name.clone(), args: t.args_json.clone() });
                return;
            }
            if !thinking.is_empty() {
                let _ = chan.send(StreamChunk::Thinking { content: thinking.to_string() });
            }
            if !token_delta.is_empty() {
                let _ = chan.send(StreamChunk::Token { content: token_delta.to_string() });
            }
        },
        &token,
    )
    .await;

    state
        .ai_cancels
        .lock()
        .expect("ai_cancels poisoned")
        .remove(&request_id);

    match result {
        Ok(full) => {
            let _ = on_chunk.send(StreamChunk::Done { full: full.clone() });
            Ok(full)
        }
        Err(e) => {
            let msg = e.to_string();
            if msg == byte_memos_core::ai::stream::CANCELLED_MSG {
                Err(byte_memos_core::ai::stream::CANCELLED_MSG.to_string())
            } else {
                let _ = on_chunk.send(StreamChunk::Error { message: msg.clone() });
                Err(msg)
            }
        }
    }
}

/// 供 lib.rs 组装 AppState 的取消注册表初值
pub fn empty_cancel_registry() -> HashMap<String, CancellationToken> {
    HashMap::new()
}
