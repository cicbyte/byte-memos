//! REST API 共享状态、辅助函数与跨子模块复用的 Query/Body 结构

use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;

use crate::ProjectService;

/// API 共享状态：ProjectService + API Key 配置
#[derive(Clone)]
pub struct ApiState {
    pub svc: Arc<ProjectService>,
    pub api_key: String,
    pub allow_local_no_key: bool,
}

// ============ 辅助 ============

pub(super) fn err_resp(e: impl std::fmt::Display) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": e.to_string()})),
    )
        .into_response()
}

pub(super) fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "Not found"})),
    )
        .into_response()
}

// ============ 跨子模块共享的 Query / Body 结构 ============

#[derive(Deserialize)]
pub(super) struct SearchQ {
    pub q: String,
    #[serde(default = "default_limit_20")]
    pub limit: i64,
}
fn default_limit_20() -> i64 {
    20
}
