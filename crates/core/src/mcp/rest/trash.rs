//! 回收站 handler（列表 / 清空 / 恢复 / 彻底删除）
//!
//! 知识库五类移除后：restore_item / purge_item 仅支持 type=memo。

use axum::{
    extract::{Path, State as AxumState},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};

use super::state::{err_resp, ApiState};

pub(super) async fn list_trashed(AxumState(state): AxumState<ApiState>) -> Response {
    match state.svc.list_trashed() {
        Ok(list) => Json(list).into_response(),
        Err(e) => err_resp(e),
    }
}

pub(super) async fn empty_trash(AxumState(state): AxumState<ApiState>) -> Response {
    match state.svc.empty_trash() {
        Ok(_) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => err_resp(e),
    }
}

pub(super) async fn restore_item(
    AxumState(state): AxumState<ApiState>,
    Path((item_type, id)): Path<(String, String)>,
) -> Response {
    let result = match item_type.as_str() {
        "memo" => state.svc.restore_memo(&id),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid type, only 'memo' is supported"})),
            )
                .into_response()
        }
    };
    match result {
        Ok(_) => Json(serde_json::json!({"restored": id})).into_response(),
        Err(e) => err_resp(e),
    }
}

pub(super) async fn purge_item(
    AxumState(state): AxumState<ApiState>,
    Path((item_type, id)): Path<(String, String)>,
) -> Response {
    let result = match item_type.as_str() {
        "memo" => state.svc.purge_memo(&id),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid type, only 'memo' is supported"})),
            )
                .into_response()
        }
    };
    match result {
        Ok(_) => Json(serde_json::json!({"purged": id})).into_response(),
        Err(e) => err_resp(e),
    }
}
