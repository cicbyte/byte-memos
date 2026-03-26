//! 回收站命令：列表 / 清空
//!
//! 备忘录的恢复 / 彻底删除由 commands::memos 提供（restore_memo / purge_memo）；
//! 本模块只保留跨类型的「列表 + 清空」聚合视图。

use crate::state::AppState;
use byte_memos_core::model::TrashedItem;

#[tauri::command]
pub fn list_trashed(state: tauri::State<'_, AppState>) -> Result<Vec<TrashedItem>, String> {
    state.svc().list_trashed().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn empty_trash(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.svc().empty_trash().map_err(|e| e.to_string())
}
