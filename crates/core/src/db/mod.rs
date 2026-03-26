//! 数据访问层
//!
//! 持有一个 [`rusqlite::Connection`]，封装全部 SQL 操作。
//! 本模块是 SQL 的唯一接触面：service 层只调用 db 方法，不直接写 SQL。
//!
//! byte-memos 笔记应用：保留 memos CRUD/搜索 + 共享的
//! tags/categories 与统计搜索。共享辅助（WHERE 构建器、批量关联加载、时间戳、
//! FTS 查询构造）位于 [`common`]，`now_iso` 经 `pub use` 重导出，
//! 保持 `crate::db::now_iso` 路径不变。

pub mod schema;

mod common;
mod search;
mod memos;
mod relations;
mod versions;
pub mod templates;
pub mod attachments;

#[cfg(test)]
mod tests;

pub use common::now_iso;

use std::path::Path;
use std::sync::Mutex;

use crate::model::*;
use crate::{CoreError, CoreResult};
use rusqlite::{params, Connection};

use common::upsert_named;

/// 数据库句柄（线程安全的单连接，CLI/桌面端本地短查询足够）
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// 打开（或创建）指定路径的 SQLite 库，并运行全部迁移
    pub fn open<P: AsRef<Path>>(path: P) -> CoreResult<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        schema::run_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 内存库（仅供测试）
    #[cfg(test)]
    pub fn open_in_memory_for_test() -> CoreResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        schema::run_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 获取数据库连接锁（供 service 层直接执行 SQL）
    pub fn lock(&self) -> CoreResult<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|e| CoreError::Other(anyhow::anyhow!("db mutex poisoned: {e}")))
    }

    // ============ shortId 查重 ============

    /// 该 shortId 是否已存在（查 memo 表）
    pub fn short_id_exists(&self, short_id: &str) -> CoreResult<bool> {
        let conn = self.lock()?;
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memo WHERE short_id = ?",
            params![short_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    // ============ 标签 / 分类（按名取或建）============

    /// 取得或创建 tag，返回其 id
    pub fn upsert_tag(&self, name: &str) -> CoreResult<i64> {
        let conn = self.lock()?;
        upsert_named(&conn, "INSERT OR IGNORE INTO tags(name) VALUES(?)", "tags", name)
    }

    /// 取得或创建带父标签的 tag，返回其 id
    ///
    /// 如果 parent_name 为 Some，则先创建父标签，再创建子标签。
    /// 支持多层级：#工作/项目A → 父标签"工作"，子标签"项目A"
    pub fn upsert_tag_with_parent(&self, name: &str, parent_name: Option<&str>, color: Option<&str>) -> CoreResult<i64> {
        let conn = self.lock()?;

        // 如果有父标签，先创建父标签
        let parent_id = if let Some(pname) = parent_name {
            let pid = upsert_named(&conn, "INSERT OR IGNORE INTO tags(name) VALUES(?)", "tags", pname)?;
            Some(pid)
        } else {
            None
        };

        // 创建或获取当前标签
        let id = upsert_named(&conn, "INSERT OR IGNORE INTO tags(name) VALUES(?)", "tags", name)?;

        // 更新 parent_id 和 color
        if parent_id.is_some() || color.is_some() {
            conn.execute(
                "UPDATE tags SET parent_id = COALESCE(?, parent_id), color = COALESCE(?, color) WHERE id = ?",
                rusqlite::params![parent_id, color, id],
            )?;
        }

        Ok(id)
    }

    /// 列出所有标签（树形结构）
    pub fn list_tags_tree(&self) -> CoreResult<Vec<TagTreeNode>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.parent_id, t.color,
                (SELECT COUNT(*) FROM memo_tag mt
                   JOIN memo m ON m.short_id = mt.memo_id
                 WHERE mt.tag_id = t.id AND m.deleted_at IS NULL) AS cnt
             FROM tags t
             ORDER BY t.parent_id NULLS FIRST, t.name",
        )?;
        let rows: Vec<TagTreeNode> = stmt
            .query_map([], |r| {
                Ok(TagTreeNode {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    parent_id: r.get(2)?,
                    color: r.get(3)?,
                    count: r.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    pub fn upsert_category(&self, name: &str) -> CoreResult<i64> {
        let conn = self.lock()?;
        upsert_named(
            &conn,
            "INSERT OR IGNORE INTO project_categories(name) VALUES(?)",
            "project_categories",
            name,
        )
    }

    /// 列出全部分类（含每分类下的笔记数），按笔记数降序、名称升序
    pub fn list_categories(&self) -> CoreResult<Vec<Category>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT c.id, c.name,
                (SELECT COUNT(*) FROM memo m
                 WHERE m.category_id = c.id AND m.deleted_at IS NULL) AS memo_cnt
             FROM project_categories c
             ORDER BY memo_cnt DESC, c.name ASC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Category {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    memo_count: r.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 重命名分类（校验新名不与现有分类重名）
    pub fn rename_category(&self, id: i64, new_name: &str) -> CoreResult<()> {
        let conn = self.lock()?;
        let dup: i64 = conn.query_row(
            "SELECT COUNT(*) FROM project_categories WHERE name = ? AND id != ?",
            params![new_name, id],
            |r| r.get(0),
        )?;
        if dup > 0 {
            return Err(CoreError::AlreadyExists(format!("分类名「{new_name}」已存在")));
        }
        let affected = conn.execute(
            "UPDATE project_categories SET name = ? WHERE id = ?",
            params![new_name, id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound(format!("分类 id={id}")));
        }
        Ok(())
    }

    /// 删除分类（memo.category_id 有 ON DELETE SET NULL，关联笔记自动解绑）
    pub fn delete_category(&self, id: i64) -> CoreResult<()> {
        let conn = self.lock()?;
        let affected = conn.execute(
            "DELETE FROM project_categories WHERE id = ?",
            params![id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound(format!("分类 id={id}")));
        }
        Ok(())
    }

    /// 分类总数
    pub fn category_count(&self) -> CoreResult<i64> {
        let conn = self.lock()?;
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM project_categories", [], |r| r.get(0))?;
        Ok(n)
    }
}
