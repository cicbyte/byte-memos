//! 附件服务层。

use std::path::Path;

use super::*;

impl super::ProjectService {
    /// 上传附件（DB 存储模式：文件内容存入数据库）
    pub fn upload_attachment_db(
        &self,
        memo_short_id: &str,
        filename: &str,
        mime_type: &str,
        content: &[u8],
    ) -> CoreResult<i64> {
        // 验证笔记存在
        self.get_memo(memo_short_id)?;
        self.db.create_attachment_db(memo_short_id, filename, mime_type, content)
    }

    /// 上传附件（FILE 存储模式：文件存本地，数据库记录路径）
    pub fn upload_attachment_file(
        &self,
        memo_short_id: &str,
        filename: &str,
        source_path: &Path,
    ) -> CoreResult<i64> {
        // 验证笔记存在
        self.get_memo(memo_short_id)?;
        
        // 确定存储路径
        let attachments_dir = self.data_dir().join("attachments").join(memo_short_id);
        std::fs::create_dir_all(&attachments_dir)?;
        
        let dest_path = attachments_dir.join(filename);
        std::fs::copy(source_path, &dest_path)?;
        
        let file_size = std::fs::metadata(&dest_path)?.len() as i64;
        let mime_type = mime_guess_from_filename(filename);
        let relative_path = format!("attachments/{}/{}", memo_short_id, filename);
        
        self.db.create_attachment_file(memo_short_id, filename, &relative_path, file_size, &mime_type)
    }

    /// 获取附件信息
    pub fn get_attachment(&self, id: i64) -> CoreResult<Option<crate::db::attachments::Attachment>> {
        self.db.get_attachment(id)
    }

    /// 获取附件内容（DB 存储模式）
    pub fn get_attachment_content(&self, id: i64) -> CoreResult<Option<Vec<u8>>> {
        self.db.get_attachment_blob(id)
    }

    /// 列出笔记的所有附件
    pub fn list_attachments(&self, memo_short_id: &str) -> CoreResult<Vec<crate::db::attachments::Attachment>> {
        self.db.list_attachments(memo_short_id)
    }

    /// 删除附件
    pub fn delete_attachment(&self, id: i64) -> CoreResult<()> {
        // 获取附件信息以删除文件
        if let Some(att) = self.db.get_attachment(id)? {
            if att.storage_type == "FILE" && !att.file_path.is_empty() {
                let full_path = self.data_dir().join(&att.file_path);
                if full_path.exists() {
                    let _ = std::fs::remove_file(full_path);
                }
            }
        }
        self.db.delete_attachment(id)
    }
}

/// 根据文件名猜测 MIME 类型
fn mime_guess_from_filename(filename: &str) -> String {
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "json" => "application/json",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }.to_string()
}
