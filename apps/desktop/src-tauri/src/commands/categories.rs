//! 分类命令

use crate::state::AppState;
use byte_memos_core::model::*;

/// 列出全部分类（含项目数）
#[tauri::command]
pub fn list_categories(state: tauri::State<'_, AppState>) -> Result<Vec<Category>, String> {
    state.svc().list_categories().map_err(|e| e.to_string())
}

/// 新建分类（重名则返回已有 id）
#[tauri::command]
pub fn create_category(state: tauri::State<'_, AppState>, name: String) -> Result<i64, String> {
    state.svc().create_category(&name).map_err(|e| e.to_string())
}

/// 重命名分类
#[tauri::command]
pub fn rename_category(state: tauri::State<'_, AppState>, id: i64, name: String) -> Result<(), String> {
    state.svc().rename_category(id, &name).map_err(|e| e.to_string())
}

/// 删除分类（关联项目 category_id 自动置空）
#[tauri::command]
pub fn delete_category(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    state.svc().delete_category(id).map_err(|e| e.to_string())
}
