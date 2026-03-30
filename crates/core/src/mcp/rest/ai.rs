//! AI handler（D3：移动端 AI 对话）
//!
//! - `GET  /ai/providers`  供应商与模型列表（不含 api_key）
//! - `POST /ai/chat/stream` SSE 流式对话（delta / thinking / done / error 事件）
//!
//! 供应商沿用桌面端配置（api_key 只在 Rust 侧解密使用，绝不回传）。
//! 请求体 `model` 可覆盖该次使用的模型（移动端切换模型用）。

use axum::{
    extract::State as AxumState,
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Json,
    },
};
use futures::stream::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use std::task::Poll;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::state::{err_resp, ApiState};

// ============ 供应商列表 ============

#[derive(serde::Serialize)]
struct AiProviderDto {
    id: i64,
    name: String,
    provider_key: String,
    model: String,
    models: Vec<String>,
    is_enabled: bool,
}

pub(super) async fn list_ai_providers(AxumState(state): AxumState<ApiState>) -> axum::response::Response {
    match state.svc.ai_provider_list() {
        Ok(rows) => Json(rows
            .into_iter()
            .map(|r| AiProviderDto {
                id: r.id,
                name: r.name,
                provider_key: r.provider_key,
                model: r.model,
                models: r.models,
                is_enabled: r.is_enabled,
            })
            .collect::<Vec<_>>())
        .into_response(),
        Err(e) => err_resp(e),
    }
}

// ============ SSE 流式对话 ============

#[derive(Deserialize)]
pub(super) struct ChatMessage {
    role: String, // "user" | "assistant"
    content: String,
}

#[derive(Deserialize)]
pub(super) struct ChatStreamB {
    messages: Vec<ChatMessage>,
    #[serde(default)]
    provider_id: Option<i64>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    system: Option<String>,
}

pub(super) async fn ai_chat_stream(
    AxumState(state): AxumState<ApiState>,
    axum::Json(body): axum::Json<ChatStreamB>,
) -> axum::response::Response {
    // 解析供应商配置：指定 id 或当前启用的
    let cfg = match body.provider_id {
        Some(id) => state.svc.ai_provider_config_by_id(id).map_err(Into::into),
        None => state
            .svc
            .enabled_ai_provider_config()
            .map_err(Into::into)
            .and_then(|o| o.ok_or_else(|| anyhow::anyhow!("没有已启用的 AI 供应商，请先在桌面端配置"))),
    };
    let mut cfg = match cfg {
        Ok(c) => c,
        Err(e) => return err_resp(e),
    };
    if let Some(m) = body.model.as_deref() {
        if !m.trim().is_empty() {
            cfg.model = m.trim().to_string();
        }
    }

    // 最后一条必须是 user 输入；之前的进 history
    let mut history: Vec<(bool, String)> = Vec::new();
    let mut user_input = String::new();
    let last = body.messages.len().saturating_sub(1);
    for (i, m) in body.messages.iter().enumerate() {
        let is_user = m.role == "user";
        if i == last && is_user {
            user_input = m.content.clone();
        } else {
            history.push((is_user, m.content.clone()));
        }
    }
    if user_input.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "最后一条消息必须是用户输入"})),
        )
            .into_response();
    }
    // 防超长上下文：history 只留最近 40 条
    if history.len() > 40 {
        history.drain(..history.len() - 40);
    }
    let system = body.system.unwrap_or_else(|| {
        "你是 byte-memos 的移动端 AI 助手。回答保持简洁、用中文，适合手机阅读。".to_string()
    });

    // 事件通道：run_chat_stream 的同步增量回调 → mpsc → SSE
    let (tx, rx) = mpsc::unbounded_channel::<Event>();
    let cancel = CancellationToken::new();
    let cancel_for_task = cancel.clone();

    tokio::spawn(async move {
        let (delta_tx, mut delta_rx) = mpsc::unbounded_channel::<(String, String)>();
        let sender = delta_tx.clone();
        let mut on_delta = move |text: &str, thinking: &str| {
            let _ = sender.send((text.to_string(), thinking.to_string()));
        };
        let cfg2 = cfg.clone();
        let system2 = system.clone();
        let history2 = history.clone();
        let user2 = user_input.clone();
        let handle = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(crate::ai::stream::run_chat_stream(
                &cfg2,
                &system2,
                &history2,
                &user2,
                &mut on_delta,
                &cancel_for_task,
            ))
        });
        // delta 通道在 AI 调用结束（闭包销毁）后自然关闭 → 循环退出
        let mut client_gone = false;
        while let Some((text, thinking)) = delta_rx.recv().await {
            let ev = if !thinking.is_empty() {
                Event::default()
                    .event("thinking")
                    .json_data(serde_json::json!({"text": thinking}))
            } else {
                Event::default()
                    .event("delta")
                    .json_data(serde_json::json!({"text": text}))
            }
            .unwrap_or_else(|_| Event::default());
            if tx.send(ev).is_err() {
                // SSE 客户端已断开 → 取消上游 AI 请求
                cancel.cancel();
                client_gone = true;
                break;
            }
        }
        if client_gone {
            let _ = handle.await;
            return;
        }
        let result = handle.await.unwrap_or_else(|e| Err(anyhow::anyhow!("AI 任务失败: {e}")));
        let ev = match result {
            Ok(full) => Event::default()
                .event("done")
                .json_data(serde_json::json!({"full": full})),
            Err(e) => Event::default()
                .event("error")
                .json_data(serde_json::json!({"message": e.to_string()})),
        }
        .unwrap_or_else(|_| Event::default());
        let _ = tx.send(ev);
    });

    // rx → Stream（poll_fn 手搓，不引新依赖）
    let mut rx = rx;
    let stream = futures::stream::poll_fn(move |cx| match rx.poll_recv(cx) {
        Poll::Ready(Some(ev)) => Poll::Ready(Some(Ok::<Event, Infallible>(ev))),
        Poll::Ready(None) => Poll::Ready(None),
        Poll::Pending => Poll::Pending,
    });
    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}
