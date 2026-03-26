//! 分类 handler（列表 / 创建 / 重命名 / 删除）

use axum::{
    extract::{Path, State as AxumState},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Deserialize;

use super::state::{err_resp, ApiState};

pub(super) async fn list_categories(AxumState(state): AxumState<ApiState>) -> Response {
    match state.svc.list_categories() {
        Ok(list) => Json(list).into_response(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
pub(super) struct CreateCategoryB {
    name: String,
}

pub(super) async fn create_category(
    AxumState(state): AxumState<ApiState>,
    Json(body): Json<CreateCategoryB>,
) -> Response {
    match state.svc.create_category(&body.name) {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({"id": id, "name": body.name}))).into_response(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
pub(super) struct RenameCategoryB {
    name: String,
}

pub(super) async fn rename_category(
    AxumState(state): AxumState<ApiState>,
    Path(id): Path<i64>,
    Json(body): Json<RenameCategoryB>,
) -> Response {
    match state.svc.rename_category(id, &body.name) {
        Ok(_) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => err_resp(e),
    }
}

pub(super) async fn delete_category(
    AxumState(state): AxumState<ApiState>,
    Path(id): Path<i64>,
) -> Response {
    match state.svc.delete_category(id) {
        Ok(_) => Json(serde_json::json!({"removed": id})).into_response(),
        Err(e) => err_resp(e),
    }
}
