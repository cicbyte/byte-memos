//! 统计 / 标签 handler

use axum::{
    extract::{Query, State as AxumState},
    response::{IntoResponse, Json, Response},
};
use serde::Deserialize;

use super::state::{err_resp, ApiState};

pub(super) async fn stats_overview(AxumState(state): AxumState<ApiState>) -> Response {
    match state.svc.stats_overview() {
        Ok(s) => Json(s).into_response(),
        Err(e) => err_resp(e),
    }
}

/// 日历热力图数据源：按本地日期聚合未删除备忘数（移动端日历视图）
pub(super) async fn daily_counts(AxumState(state): AxumState<ApiState>) -> Response {
    match state.svc.list_memo_daily_counts() {
        Ok(list) => Json(list).into_response(),
        Err(e) => err_resp(e),
    }
}

#[derive(Deserialize)]
pub(super) struct TagStatsQ {
    #[serde(default = "default_limit_50")]
    limit: i64,
}
fn default_limit_50() -> i64 {
    50
}

pub(super) async fn tag_stats(
    AxumState(state): AxumState<ApiState>,
    Query(q): Query<TagStatsQ>,
) -> Response {
    match state.svc.tag_stats(q.limit) {
        Ok(list) => Json(list).into_response(),
        Err(e) => err_resp(e),
    }
}
