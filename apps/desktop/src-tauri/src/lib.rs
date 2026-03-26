//! byte-memos 桌面端 Tauri 后端
//!
//! 作为 byte-memos-core 的薄封装层：每个 `#[tauri::command]` 调用 core 的 ProjectService，
//! 桌面端复用与 CLI 完全一致的业务逻辑。
//!
//! 命令按业务域拆分到 [`commands`] 各子模块，应用状态见 [`state`]。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use std::sync::Mutex;
use byte_memos_core::ProjectService;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let svc = ProjectService::open_default().expect("无法初始化 byte-memos 数据目录");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // 全局快捷键（F13）：默认 CmdOrCtrl+Shift+B 唤起/隐藏主窗口
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    use tauri::Manager;
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state() == ShortcutState::Pressed {
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .build(),
        )
        .manage(AppState {
            svc: Mutex::new(svc),
            mcp: Mutex::new(None),
            api: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            // 系统
            commands::system::ping,
            commands::system::data_dir,
            commands::system::open_in_folder,
            commands::system::open_url,
            // 分类
            commands::categories::list_categories,
            commands::categories::create_category,
            commands::categories::rename_category,
            commands::categories::delete_category,
            commands::categories::set_category_color,
            // 统计（标签云 / 概览 / 全局搜索 / 日历日计数）
            commands::stats::tag_stats,
            commands::stats::stats_overview,
            commands::stats::search_all,
            commands::stats::memo_daily_counts,
            // 附件（byte-memos 附件管理域）
            commands::attachments::attachment_upload,
            commands::attachments::attachment_list,
            commands::attachments::attachment_list_all,
            commands::attachments::attachment_content,
            commands::attachments::attachment_delete,
            commands::attachments::attachment_save,
            // 备忘录（byte-memos 核心领域）
            commands::memos::add_memo,
            commands::memos::update_memo,
            commands::memos::list_memos,
            commands::memos::list_memos_paged,
            commands::memos::get_memo,
            commands::memos::delete_memo,
            commands::memos::toggle_memo_pinned,
            commands::memos::set_memo_visibility,
            commands::memos::search_memos,
            commands::memos::restore_memo,
            commands::memos::purge_memo,
            commands::memos::memo_history,
            commands::memos::restore_memo_version,
            commands::memos::export_memo,
            // 回收站（跨类型，仅剩备忘录条目）
            commands::trash::list_trashed,
            commands::trash::empty_trash,
            // 备份 / 数据目录 / 配置
            commands::config::backup,
            commands::config::restore,
            commands::config::set_data_dir,
            commands::config::clear_data_dir,
            commands::config::get_exclude_dirs,
            commands::config::set_exclude_dirs,
            commands::config::set_api_keys,
            // 壁纸
            commands::wallpaper::upload_wallpaper,
            commands::wallpaper::list_wallpapers,
            commands::wallpaper::delete_wallpaper,
            // MCP HTTP 服务管理
            commands::servers::start_mcp_server,
            commands::servers::stop_mcp_server,
            commands::servers::mcp_server_status,
            // REST API 服务管理
            commands::servers::start_api_server,
            commands::servers::stop_api_server,
            commands::servers::api_server_status,
        ])
        .setup(|app| {
            // 注册默认全局快捷键（CmdOrCtrl+Shift+B）；冲突时仅 warn，不阻断启动
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            if let Err(e) = app.global_shortcut().register("CmdOrCtrl+Shift+B") {
                eprintln!("[global-shortcut] 注册失败（可能被占用）：{e}");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("运行 byte-memos 桌面端时出错");
}
