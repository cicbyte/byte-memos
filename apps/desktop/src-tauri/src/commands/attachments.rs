//! 附件命令（byte-memos 附件管理域）
//!
//! 薄封装 core ProjectService 附件方法。桌面端上传统一走 FILE 存储
//! （字节落盘到 data/attachments/{short_id}/），DB blob 模式保留给 CLI。

use crate::state::AppState;
use byte_memos_core::db::attachments::{Attachment, AttachmentOverview};

/// 附件内容载荷（前端用 Blob 预览 / 落盘下载）
#[derive(serde::Serialize)]
pub struct AttachmentContent {
    pub filename: String,
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

#[tauri::command]
pub fn attachment_upload(
    state: tauri::State<'_, AppState>,
    memo_short_id: String,
    filename: String,
    mime_type: Option<String>,
    content: Vec<u8>,
) -> Result<Attachment, String> {
    state
        .svc()
        .upload_attachment_bytes(&memo_short_id, &filename, mime_type.as_deref().unwrap_or(""), &content)
        .map(|id| {
            state
                .svc()
                .get_attachment(id)
                .ok()
                .flatten()
                .expect("刚创建的附件必然存在")
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn attachment_list(
    state: tauri::State<'_, AppState>,
    memo_short_id: String,
) -> Result<Vec<Attachment>, String> {
    state
        .svc()
        .list_attachments(&memo_short_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn attachment_list_all(state: tauri::State<'_, AppState>) -> Result<Vec<AttachmentOverview>, String> {
    state.svc().list_all_attachments().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn attachment_content(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<AttachmentContent, String> {
    state
        .svc()
        .read_attachment_content(id)
        .map_err(|e| e.to_string())?
        .map(|(att, bytes)| AttachmentContent {
            filename: att.filename,
            mime_type: att.mime_type,
            bytes,
        })
        .ok_or_else(|| format!("附件 {id} 不存在"))
}

#[tauri::command]
pub fn attachment_delete(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    state.svc().delete_attachment(id).map_err(|e| e.to_string())
}

/// 附件另存为（落盘到用户选择路径，返回写入路径）
#[tauri::command]
pub fn attachment_save(
    state: tauri::State<'_, AppState>,
    id: i64,
    path: String,
) -> Result<String, String> {
    let (_, bytes) = state
        .svc()
        .read_attachment_content(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("附件 {id} 不存在"))?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path)
}
