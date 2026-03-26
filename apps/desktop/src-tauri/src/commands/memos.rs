//! 备忘录命令（byte-memos 核心领域）
//!
//! 每个 `#[tauri::command]` 是 core ProjectService 的薄封装，
//! 桌面端复用与 CLI 完全一致的备忘录业务逻辑。

use crate::state::AppState;
use byte_memos_core::model::*;

#[tauri::command]
pub fn add_memo(
    state: tauri::State<'_, AppState>,
    title: String,
    content: String,
    visibility: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Memo, String> {
    let input = AddMemoInput {
        title,
        content,
        visibility: visibility.unwrap_or_else(|| "PRIVATE".to_string()),
        category,
        tags: tags.unwrap_or_default(),
    };
    state.svc().add_memo(&input).map_err(|e| e.to_string())
}

/// 编辑用更新：前端传完整表单字段（覆盖式）
#[tauri::command]
pub fn update_memo(
    state: tauri::State<'_, AppState>,
    short_id: String,
    title: String,
    content: String,
    visibility: Option<String>,
    category: Option<Option<String>>,
    tags: Vec<String>,
    pinned: bool,
) -> Result<Memo, String> {
    let input = UpdateMemoInput {
        title: Some(title),
        content: Some(content),
        visibility,
        category,
        tags: Some(tags),
        pinned: Some(pinned),
    };
    state
        .svc()
        .update_memo(&short_id, &input)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_memos(
    state: tauri::State<'_, AppState>,
    category: Option<String>,
    tag: Option<String>,
    pinned_only: Option<bool>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Memo>, String> {
    let filter = MemoFilter {
        category,
        tag,
        pinned_only: pinned_only.unwrap_or(false),
        limit,
        offset,
    };
    state.svc().list_memos(&filter).map_err(|e| e.to_string())
}

/// 分页列出备忘录（items + total，供前端无限滚动）
#[tauri::command]
pub fn list_memos_paged(
    state: tauri::State<'_, AppState>,
    category: Option<String>,
    tag: Option<String>,
    pinned_only: Option<bool>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<PagedResult<Memo>, String> {
    let filter = MemoFilter {
        category,
        tag,
        pinned_only: pinned_only.unwrap_or(false),
        limit,
        offset,
    };
    state
        .svc()
        .list_memos_paged(&filter)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_memo(state: tauri::State<'_, AppState>, short_id: String) -> Result<Memo, String> {
    state.svc().get_memo(&short_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memo(state: tauri::State<'_, AppState>, short_id: String) -> Result<(), String> {
    state.svc().delete_memo(&short_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_memo_pinned(
    state: tauri::State<'_, AppState>,
    short_id: String,
) -> Result<Memo, String> {
    state
        .svc()
        .toggle_memo_pinned(&short_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_memo_visibility(
    state: tauri::State<'_, AppState>,
    short_id: String,
    visibility: String,
) -> Result<Memo, String> {
    state
        .svc()
        .set_memo_visibility(&short_id, &visibility)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_memos(
    state: tauri::State<'_, AppState>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<MemoSearchHit>, String> {
    state
        .svc()
        .search_memos(&query, limit.unwrap_or(20), 0)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_memo(state: tauri::State<'_, AppState>, short_id: String) -> Result<(), String> {
    state
        .svc()
        .restore_memo(&short_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn purge_memo(state: tauri::State<'_, AppState>, short_id: String) -> Result<(), String> {
    state.svc().purge_memo(&short_id).map_err(|e| e.to_string())
}

/// 版本历史（内容变更自动快照，按时间倒序）
#[tauri::command]
pub fn memo_history(
    state: tauri::State<'_, AppState>,
    short_id: String,
) -> Result<Vec<byte_memos_core::service::MemoVersionInfo>, String> {
    state
        .svc()
        .list_versions(&short_id)
        .map_err(|e| e.to_string())
}

/// 回滚到指定版本（写入生成新版本记录，历史只增不减）
#[tauri::command]
pub fn restore_memo_version(
    state: tauri::State<'_, AppState>,
    short_id: String,
    version_id: i64,
) -> Result<Memo, String> {
    state
        .svc()
        .restore_version(&short_id, version_id)
        .map_err(|e| e.to_string())
}

/// 单篇导出：生成 markdown / html 内容并落盘到指定路径，返回写入路径
#[tauri::command]
pub fn export_memo(
    state: tauri::State<'_, AppState>,
    short_id: String,
    format: String,
    path: String,
) -> Result<String, String> {
    let content = match format.as_str() {
        "html" => state.svc().export_memo_html(&short_id),
        _ => state.svc().export_memo_markdown(&short_id),
    }
    .map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path)
}
