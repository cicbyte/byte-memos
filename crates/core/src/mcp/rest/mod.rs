//! REST API 服务（axum）
//!
//! 标准 HTTP REST 接口，供第三方应用对接备忘录库。
//! 端点前缀 `/api/v1/`，支持 API Key 认证（X-API-Key 请求头）。
//!
//! 和 MCP server 共享同一套 ProjectService，但返回标准 JSON 而非 Markdown。
//!
//! 各资源域 handler 按域拆到子模块（memos / categories / stats / trash）；
//! Swagger/OpenAPI 在 `docs`；共享状态在 `state`。`build_routes` 在本模块用 `use`
//! 把各 handler 引入作用域，函数体保持不变。

mod memos;
mod categories;
mod docs;
mod state;
mod stats;
mod trash;
mod attachments;
#[cfg(feature = "ai")]
mod ai;

use std::sync::Arc;

use axum::{
    extract::{Path, State as AxumState},
    http::{header, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post, put},
    Router,
};

use crate::ProjectService;

pub use state::ApiState;

// 把各子模块 handler 引入作用域，build_routes 函数体保持原样
use memos::{
    add_memo, get_memo, list_memos, remove_memo, search_memos,
    set_memo_visibility, toggle_memo_pinned, update_memo,
};
use categories::{create_category, delete_category, list_categories, rename_category};
use docs::{openapi_spec, swagger_ui};
use stats::{stats_overview, tag_stats};
use trash::{empty_trash, list_trashed, purge_item, restore_item};
use attachments::{delete_attachment_rest, download_attachment, list_attachments, upload_attachment};
#[cfg(feature = "ai")]
use ai::{ai_chat_stream, list_ai_providers};

// ============ 启动函数 ============

/// 启动 REST API HTTP 服务（managed，返回 JoinHandle，外部用 cancellation token 停止）
pub fn serve_api_http_managed(
    svc: Arc<ProjectService>,
    host: &str,
    port: u16,
    api_key: String,
    allow_local_no_key: bool,
    _cancel: tokio_util::sync::CancellationToken,
) -> anyhow::Result<tokio::task::JoinHandle<()>> {
    let state = ApiState {
        svc,
        api_key,
        allow_local_no_key,
    };
    let app = api_router(state);
    let addr = format!("{host}:{port}");
    let rt = tokio::runtime::Handle::current();
    let handle = rt.spawn(async move {
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                tracing::info!("REST API server listening on http://{addr}/api/v1");
                let _ = axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await;
            }
            Err(e) => tracing::error!("REST API bind {addr} 失败：{e}"),
        }
    });
    Ok(handle)
}

/// API Key 中间件（简化版：在 router 层用 layer 挂载）
///
/// `allow_local_no_key=true` 时，本地回环（127.0.0.1 / ::1）免 API Key。
/// 用 ConnectInfo（真实连接 IP）判断，而非 host 头（host 头可被客户端伪造）。
async fn api_key_middleware(
    axum::extract::State(state): axum::extract::State<ApiState>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    if state.api_key.is_empty() {
        return next.run(req).await;
    }
    // allow_local_no_key：本地回环免 API Key（基于真实连接 IP，防 host 头伪造）
    if state.allow_local_no_key && addr.ip().is_loopback() {
        return next.run(req).await;
    }
    // 检查 X-API-Key 头
    let provided = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided == state.api_key {
        return next.run(req).await;
    }
    // 检查 Bearer token
    let auth = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if let Some(token) = auth.strip_prefix("Bearer ") {
        if token == state.api_key {
            return next.run(req).await;
        }
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "Invalid or missing API key. Provide X-API-Key header."})),
    )
        .into_response()
}

/// 构建带认证中间件的完整 router（docs/openapi.json 不需认证）
fn api_router(state: ApiState) -> Router {
    let routes = build_routes().with_state(state.clone());
    Router::new()
        // API 端点（需认证）
        .nest("/api/v1", routes)
        .route_layer(from_fn_with_state(state.clone(), api_key_middleware))
        // Swagger UI + OpenAPI spec（无需认证）
        .route("/api/v1/docs", get(swagger_ui))
        .route("/api/v1/openapi.json", get(openapi_spec))
        // 公开分享页（无需认证；仅 PUBLIC 可见，其余 404）——链接分享落地页
        .route("/m/{id}", get(public_memo_page).with_state(state.clone()))
}

/// 公开备忘分享页：极简无依赖 HTML（内容为 Markdown 源文，pre-wrap 呈现）
async fn public_memo_page(AxumState(state): AxumState<ApiState>, Path(id): Path<String>) -> Response {
    let page = |title: &str, body: String, status: StatusCode| {
        let html = format!(
            "<!doctype html><html lang=\"zh\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\">\
<title>{t}</title><style>\
:root{{color-scheme:light dark}}body{{margin:0;font:16px/1.8 -apple-system,'Segoe UI',sans-serif;\
background:#f6f4ef;color:#1f2328;display:flex;justify-content:center;min-height:100vh}}\
main{{max-width:680px;padding:48px 24px;width:100%;box-sizing:border-box}}\
h1{{font-size:1.45em;margin:0 0 .4em}}\
.meta{{font-size:.82em;color:#8a8f98;margin-bottom:2.2em}}\
pre.content{{white-space:pre-wrap;word-break:break-word;font:inherit;margin:0}}\
.tag{{display:inline-block;margin-right:.6em;color:#0e7a6f;font-size:.85em}}\
footer{{margin-top:4em;font-size:.75em;color:#b0b4bb}}\
@media(prefers-color-scheme:dark){{body{{background:#161817;color:#e6e8ee}}.meta{{color:#7d828c}}footer{{color:#565b63}}}}\
</style></head><body><main>{body}\
<footer>byte-memos · 公开分享</footer></main></body></html>",
            t = html_escape(title),
            body = body,
        );
        ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], (status, html)).into_response()
    };

    let m = match state.svc.get_memo(&id) {
        Ok(m) => m,
        Err(_) => {
            return page(
                "404",
                "<h1>404</h1><p class=\"meta\">备忘不存在或未公开</p>".to_string(),
                StatusCode::NOT_FOUND,
            )
        }
    };
    if m.visibility != "PUBLIC" {
        return page(
            "404",
            "<h1>404</h1><p class=\"meta\">这条备忘没有公开</p>".to_string(),
            StatusCode::NOT_FOUND,
        );
    }
    let tags = m.tags.iter().map(|t| format!("<span class=\"tag\">#{}</span>", html_escape(t))).collect::<Vec<_>>().join("");
    let body = format!(
        "<h1>{}</h1><p class=\"meta\">{}{}</p><pre class=\"content\">{}</pre>",
        html_escape(&m.title),
        html_escape(&m.created_at),
        if tags.is_empty() { String::new() } else { format!(" · {}", tags) },
        html_escape(&m.content),
    );
    page(if m.title.is_empty() { "分享的备忘" } else { &m.title }, body, StatusCode::OK)
}

/// 最小 HTML 转义（内容与标题都过这层，防注入）
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\"', "&quot;")
}

fn build_routes() -> Router<ApiState> {
    let routes = Router::new()
        // 备忘录
        .route("/memos", get(list_memos).post(add_memo))
        .route("/memos/{id}", get(get_memo).put(update_memo).delete(remove_memo))
        .route("/memos/{id}/toggle-pinned", post(toggle_memo_pinned))
        .route("/memos/{id}/visibility", post(set_memo_visibility))
        // 附件（D2：移动端图片收纳）
        .route("/memos/{id}/attachments", get(list_attachments).post(upload_attachment))
        .route("/attachments/{att_id}", get(download_attachment).delete(delete_attachment_rest))
        // 分类
        .route("/categories", get(list_categories).post(create_category))
        .route("/categories/{id}", put(rename_category).delete(delete_category))
        // 统计 / 标签
        .route("/stats", get(stats_overview))
        .route("/tags", get(tag_stats))
        // 搜索
        .route("/search/memos", get(search_memos))
        // 回收站
        .route("/trash", get(list_trashed).post(empty_trash))
        .route("/trash/{type}/{id}/restore", post(restore_item))
        .route("/trash/{type}/{id}", delete(purge_item));
    // AI（D3：移动端对话，SSE 流式；需要 ai feature）
    #[cfg(feature = "ai")]
    let routes = routes
        .route("/ai/providers", get(list_ai_providers))
        .route("/ai/chat/stream", post(ai_chat_stream));
    routes
}
