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

use std::sync::Arc;

use axum::{
    http::StatusCode,
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
}

fn build_routes() -> Router<ApiState> {
    Router::new()
        // 备忘录
        .route("/memos", get(list_memos).post(add_memo))
        .route("/memos/{id}", get(get_memo).put(update_memo).delete(remove_memo))
        .route("/memos/{id}/toggle-pinned", post(toggle_memo_pinned))
        .route("/memos/{id}/visibility", post(set_memo_visibility))
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
        .route("/trash/{type}/{id}", delete(purge_item))
}
