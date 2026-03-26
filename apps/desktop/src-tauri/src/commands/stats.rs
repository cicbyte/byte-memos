//! 通用统计与搜索命令
//!
//! 备忘录的标签云（tags 页 / dashboard）、全局搜索（Navbar）依赖这些命令。

use byte_memos_core::model::{StatsOverview, TagStat, UnifiedSearchHit};
use tauri::State;

use crate::state::AppState;

/// 标签使用统计（标签云数据源）
#[tauri::command]
pub fn tag_stats(state: State<'_, AppState>, limit: Option<i64>) -> Result<Vec<TagStat>, String> {
    state
        .svc()
        .tag_stats(limit.unwrap_or(50))
        .map_err(|e| e.to_string())
}

/// 统计概览（各类计数 + 存储占用）
#[tauri::command]
pub fn stats_overview(state: State<'_, AppState>) -> Result<StatsOverview, String> {
    state.svc().stats_overview().map_err(|e| e.to_string())
}

/// 全局搜索（跨类型统一搜索，Navbar 全局搜索框数据源）
#[tauri::command]
pub fn search_all(
    state: State<'_, AppState>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<UnifiedSearchHit>, String> {
    state
        .svc()
        .search_all(&query, limit.unwrap_or(20), 0)
        .map_err(|e| e.to_string())
}
