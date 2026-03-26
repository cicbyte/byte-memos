//! 备忘录 handler（列表 / 详情 / 创建 / 更新 / 删除 / 置顶 / 可见性 / 搜索）

use axum::{
    extract::{Path, Query, State as AxumState},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Deserialize;

use crate::model::*;

use super::state::{err_resp, not_found, ApiState, SearchQ};

#[derive(Deserialize)]
pub(super) struct ListMemosQ {
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    pinned_only: Option<bool>,
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    offset: Option<i64>,
}

pub(super) async fn list_memos(
    AxumState(state): AxumState<ApiState>,
    Query(q): Query<ListMemosQ>,
) -> Response {
    let filter = MemoFilter {
        tag: q.tag,
        category: q.category,
        pinned_only: q.pinned_only.unwrap_or(false),
        limit: q.limit,
        offset: q.offset,
        ..Default::default()
    };
    match state.svc.list_memos(&filter) {
        Ok(list) => Json(list).into_response(),
        Err(e) => err_resp(e),
    }
}

pub(super) async fn get_memo(
    AxumState(state): AxumState<ApiState>,
    Path(id): Path<String>,
) -> Response {
    match state.svc.get_memo(&id) {
        Ok(m) => Json(m).into_response(),
        Err(crate::CoreError::NotFound(_)) => not_found(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
pub(super) struct AddMemoB {
    #[serde(default)]
    title: Option<String>,
    content: String,
    #[serde(default = "default_visibility")]
    visibility: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

fn default_visibility() -> String {
    "PRIVATE".to_string()
}

pub(super) async fn add_memo(
    AxumState(state): AxumState<ApiState>,
    Json(body): Json<AddMemoB>,
) -> Response {
    let input = AddMemoInput {
        title: body.title.unwrap_or_default(),
        content: body.content,
        visibility: body.visibility,
        category: body.category,
        tags: body.tags,
    };
    match state.svc.add_memo(&input) {
        Ok(m) => (StatusCode::CREATED, Json(m)).into_response(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
pub(super) struct UpdateMemoB {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    visibility: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    pinned: Option<bool>,
}

pub(super) async fn update_memo(
    AxumState(state): AxumState<ApiState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateMemoB>,
) -> Response {
    // 未提供字段保留原值
    let cur = match state.svc.get_memo(&id) {
        Ok(m) => m,
        Err(crate::CoreError::NotFound(_)) => return not_found(),
        Err(e) => return err_resp(e),
    };
    let input = UpdateMemoInput {
        title: body.title.or(Some(cur.title)),
        content: body.content.or(Some(cur.content)),
        visibility: body.visibility.or(Some(cur.visibility)),
        category: Some(body.category.or(cur.category)),
        tags: body.tags.or(Some(cur.tags)),
        pinned: body.pinned.or(Some(cur.pinned)),
    };
    match state.svc.update_memo(&id, &input) {
        Ok(m) => Json(m).into_response(),
        Err(e) => err_resp(e),
    }
}

pub(super) async fn remove_memo(
    AxumState(state): AxumState<ApiState>,
    Path(id): Path<String>,
) -> Response {
    match state.svc.delete_memo(&id) {
        Ok(_) => Json(serde_json::json!({ "removed": id })).into_response(),
        Err(e) => err_resp(e),
    }
}

pub(super) async fn toggle_memo_pinned(
    AxumState(state): AxumState<ApiState>,
    Path(id): Path<String>,
) -> Response {
    match state.svc.toggle_memo_pinned(&id) {
        Ok(m) => Json(m).into_response(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
pub(super) struct VisibilityB {
    visibility: String,
}

pub(super) async fn set_memo_visibility(
    AxumState(state): AxumState<ApiState>,
    Path(id): Path<String>,
    Json(body): Json<VisibilityB>,
) -> Response {
    match state.svc.set_memo_visibility(&id, &body.visibility) {
        Ok(m) => Json(m).into_response(),
        Err(e) => err_resp(e),
    }
}

pub(super) async fn search_memos(
    AxumState(state): AxumState<ApiState>,
    Query(q): Query<SearchQ>,
) -> Response {
    match state.svc.search_memos(&q.q, q.limit, 0) {
        Ok(hits) => Json(hits).into_response(),
        Err(e) => err_resp(e),
    }
}
