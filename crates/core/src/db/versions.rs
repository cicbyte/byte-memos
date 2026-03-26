//! 笔记版本历史数据库操作。

use rusqlite::params;

use crate::CoreResult;

use super::Database;
use super::now_iso;

/// 版本记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoVersion {
    /// 版本ID
    pub id: i64,
    /// 笔记ID
    pub memo_id: i64,
    /// 该版本的内容
    pub content: String,
    /// 创建时间
    pub created_at: String,
}

impl Database {
    /// 创建版本快照（保存笔记时自动调用）
    pub fn create_version(&self, memo_id: i64, content: &str) -> CoreResult<i64> {
        let conn = self.lock()?;
        let now = now_iso();
        conn.execute(
            "INSERT INTO memo_version(memo_id, content, created_at) VALUES (?, ?, ?)",
            params![memo_id, content, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 获取笔记的版本历史（按时间倒序）
    pub fn list_versions(&self, memo_id: i64) -> CoreResult<Vec<MemoVersion>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, memo_id, content, created_at
             FROM memo_version
             WHERE memo_id = ?
             ORDER BY created_at DESC",
        )?;
        let rows: Vec<MemoVersion> = stmt
            .query_map(params![memo_id], |r| {
                Ok(MemoVersion {
                    id: r.get(0)?,
                    memo_id: r.get(1)?,
                    content: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 获取指定版本
    pub fn get_version(&self, version_id: i64) -> CoreResult<Option<MemoVersion>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, memo_id, content, created_at
             FROM memo_version
             WHERE id = ?",
        )?;
        let mut rows = stmt.query_map(params![version_id], |r| {
            Ok(MemoVersion {
                id: r.get(0)?,
                memo_id: r.get(1)?,
                content: r.get(2)?,
                created_at: r.get(3)?,
            })
        })?;
        match rows.next() {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// 获取笔记的整数ID（用于版本历史关联）
    pub fn get_memo_int_id(&self, short_id: &str) -> CoreResult<Option<i64>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT id FROM memo WHERE short_id = ?")?;
        let mut rows = stmt.query_map(params![short_id], |r| r.get(0))?;
        match rows.next() {
            Some(Ok(id)) => Ok(Some(id)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// 删除笔记的所有版本历史
    pub fn delete_versions(&self, memo_id: i64) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "DELETE FROM memo_version WHERE memo_id = ?",
            params![memo_id],
        )?;
        Ok(())
    }
}
