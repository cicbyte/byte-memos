//! 笔记模板数据库操作。

use rusqlite::params;

use crate::CoreResult;

use super::Database;
use super::now_iso;

/// 笔记模板
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoTemplate {
    /// 模板ID
    pub id: i64,
    /// 模板名称
    pub name: String,
    /// 模板内容（Markdown）
    pub content: String,
    /// 模板描述
    pub description: String,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

impl Database {
    /// 创建模板
    pub fn create_template(&self, name: &str, content: &str, description: &str) -> CoreResult<i64> {
        let conn = self.lock()?;
        let now = now_iso();
        conn.execute(
            "INSERT INTO memo_template(name, content, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
            params![name, content, description, now, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 获取模板（按名称）
    pub fn get_template(&self, name: &str) -> CoreResult<Option<MemoTemplate>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, content, description, created_at, updated_at
             FROM memo_template
             WHERE name = ?",
        )?;
        let mut rows = stmt.query_map(params![name], |r| {
            Ok(MemoTemplate {
                id: r.get(0)?,
                name: r.get(1)?,
                content: r.get(2)?,
                description: r.get(3)?,
                created_at: r.get(4)?,
                updated_at: r.get(5)?,
            })
        })?;
        match rows.next() {
            Some(Ok(t)) => Ok(Some(t)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// 列出所有模板
    pub fn list_templates(&self) -> CoreResult<Vec<MemoTemplate>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, content, description, created_at, updated_at
             FROM memo_template
             ORDER BY name",
        )?;
        let rows: Vec<MemoTemplate> = stmt
            .query_map([], |r| {
                Ok(MemoTemplate {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    content: r.get(2)?,
                    description: r.get(3)?,
                    created_at: r.get(4)?,
                    updated_at: r.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 更新模板
    pub fn update_template(&self, name: &str, content: Option<&str>, description: Option<&str>) -> CoreResult<()> {
        let conn = self.lock()?;
        let now = now_iso();
        conn.execute(
            "UPDATE memo_template SET content = COALESCE(?, content), description = COALESCE(?, description), updated_at = ? WHERE name = ?",
            params![content, description, now, name],
        )?;
        Ok(())
    }

    /// 删除模板
    pub fn delete_template(&self, name: &str) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "DELETE FROM memo_template WHERE name = ?",
            params![name],
        )?;
        Ok(())
    }
}
