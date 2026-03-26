//! 附件数据库操作。

use rusqlite::params;

use crate::CoreResult;

use super::Database;
use super::now_iso;

/// 附件记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Attachment {
    /// 附件ID
    pub id: i64,
    /// 关联笔记 short_id
    pub memo_short_id: String,
    /// 文件名
    pub filename: String,
    /// 文件路径（FILE 模式为相对路径，DB 模式为空）
    pub file_path: String,
    /// 文件大小（字节）
    pub file_size: i64,
    /// MIME 类型
    pub mime_type: String,
    /// 存储方式：DB / FILE
    pub storage_type: String,
    /// 创建时间
    pub created_at: String,
}

/// 附件总览行（资源库页用，带备忘录存在性标记）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttachmentOverview {
    #[serde(flatten)]
    pub attachment: Attachment,
    /// 关联的备忘录已不存在（游离附件，可安全清理）
    pub orphan: bool,
}

impl Database {
    /// 创建附件记录（DB 存储模式：将文件内容存入 blob 列）
    pub fn create_attachment_db(
        &self,
        memo_short_id: &str,
        filename: &str,
        mime_type: &str,
        blob: &[u8],
    ) -> CoreResult<i64> {
        let conn = self.lock()?;
        let now = now_iso();
        let file_size = blob.len() as i64;
        conn.execute(
            "INSERT INTO attachment(memo_short_id, filename, file_path, file_size, mime_type, storage_type, blob, created_at)
             VALUES (?, ?, '', ?, ?, 'DB', ?, ?)",
            params![memo_short_id, filename, file_size, mime_type, blob, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 创建附件记录（FILE 存储模式：文件存本地，数据库记录路径）
    pub fn create_attachment_file(
        &self,
        memo_short_id: &str,
        filename: &str,
        file_path: &str,
        file_size: i64,
        mime_type: &str,
    ) -> CoreResult<i64> {
        let conn = self.lock()?;
        let now = now_iso();
        conn.execute(
            "INSERT INTO attachment(memo_short_id, filename, file_path, file_size, mime_type, storage_type, created_at)
             VALUES (?, ?, ?, ?, ?, 'FILE', ?)",
            params![memo_short_id, filename, file_path, file_size, mime_type, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 获取附件（按 ID）
    pub fn get_attachment(&self, id: i64) -> CoreResult<Option<Attachment>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, memo_short_id, filename, file_path, file_size, mime_type, storage_type, created_at
             FROM attachment WHERE id = ?",
        )?;
        let mut rows = stmt.query_map(params![id], |r| {
            Ok(Attachment {
                id: r.get(0)?,
                memo_short_id: r.get(1)?,
                filename: r.get(2)?,
                file_path: r.get(3)?,
                file_size: r.get(4)?,
                mime_type: r.get(5)?,
                storage_type: r.get(6)?,
                created_at: r.get(7)?,
            })
        })?;
        match rows.next() {
            Some(Ok(a)) => Ok(Some(a)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// 获取附件的 blob 内容（DB 存储模式）
    pub fn get_attachment_blob(&self, id: i64) -> CoreResult<Option<Vec<u8>>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT blob FROM attachment WHERE id = ? AND storage_type = 'DB'")?;
        let mut rows = stmt.query_map(params![id], |r| r.get(0))?;
        match rows.next() {
            Some(Ok(blob)) => Ok(Some(blob)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// 列出笔记的所有附件
    pub fn list_attachments(&self, memo_short_id: &str) -> CoreResult<Vec<Attachment>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, memo_short_id, filename, file_path, file_size, mime_type, storage_type, created_at
             FROM attachment WHERE memo_short_id = ? ORDER BY created_at DESC",
        )?;
        let rows: Vec<Attachment> = stmt
            .query_map(params![memo_short_id], |r| {
                Ok(Attachment {
                    id: r.get(0)?,
                    memo_short_id: r.get(1)?,
                    filename: r.get(2)?,
                    file_path: r.get(3)?,
                    file_size: r.get(4)?,
                    mime_type: r.get(5)?,
                    storage_type: r.get(6)?,
                    created_at: r.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 删除附件
    pub fn delete_attachment(&self, id: i64) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute("DELETE FROM attachment WHERE id = ?", params![id])?;
        Ok(())
    }

    /// 列出全部附件（资源库用），LEFT JOIN 标记备忘录已不存在的游离附件
    pub fn list_all_attachments(&self) -> CoreResult<Vec<AttachmentOverview>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT a.id, a.memo_short_id, a.filename, a.file_path, a.file_size,
                    a.mime_type, a.storage_type, a.created_at,
                    (m.short_id IS NULL) AS orphan
             FROM attachment a
             LEFT JOIN memo m ON m.short_id = a.memo_short_id
             ORDER BY a.created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![], |r| {
                Ok(AttachmentOverview {
                    attachment: Attachment {
                        id: r.get(0)?,
                        memo_short_id: r.get(1)?,
                        filename: r.get(2)?,
                        file_path: r.get(3)?,
                        file_size: r.get(4)?,
                        mime_type: r.get(5)?,
                        storage_type: r.get(6)?,
                        created_at: r.get(7)?,
                    },
                    orphan: r.get::<_, i64>(8)? != 0,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 删除某备忘录的全部附件记录（返回被删 id 列表，供上层清理文件）
    pub fn delete_attachments_of_memo(&self, memo_short_id: &str) -> CoreResult<Vec<i64>> {
        let conn = self.lock()?;
        let ids: Vec<i64> = {
            let mut stmt =
                conn.prepare("SELECT id FROM attachment WHERE memo_short_id = ?")?;
            let rows = stmt
                .query_map(params![memo_short_id], |r| r.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            rows
        };
        conn.execute(
            "DELETE FROM attachment WHERE memo_short_id = ?",
            params![memo_short_id],
        )?;
        Ok(ids)
    }
}
