//! 笔记关联关系（双向引用）数据库操作。

use rusqlite::params;

use crate::CoreResult;

use super::Database;
use super::now_iso;

/// 关联关系详情
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoRelation {
    /// 源笔记ID
    pub source_memo_id: String,
    /// 目标笔记ID
    pub target_memo_id: String,
    /// 关联类型：REFERENCE / COMMENT
    pub relation_type: String,
    /// 创建时间
    pub created_at: String,
}

impl Database {
    /// 创建笔记关联（双向引用）
    ///
    /// 如果关联已存在则忽略（幂等）。
    pub fn create_relation(
        &self,
        source_id: &str,
        target_id: &str,
        relation_type: &str,
    ) -> CoreResult<()> {
        let conn = self.lock()?;
        let now = now_iso();
        conn.execute(
            "INSERT OR IGNORE INTO memo_relation(source_memo_id, target_memo_id, relation_type, created_at)
             VALUES (?, ?, ?, ?)",
            params![source_id, target_id, relation_type, now],
        )?;
        Ok(())
    }

    /// 创建双向引用（REFERENCE 类型）
    ///
    /// 同时创建 A→B 和 B→A 的引用关系。
    pub fn create_bidirectional_reference(&self, memo_a: &str, memo_b: &str) -> CoreResult<()> {
        self.create_relation(memo_a, memo_b, "REFERENCE")?;
        self.create_relation(memo_b, memo_a, "REFERENCE")?;
        Ok(())
    }

    /// 删除笔记关联
    pub fn delete_relation(
        &self,
        source_id: &str,
        target_id: &str,
        relation_type: &str,
    ) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "DELETE FROM memo_relation WHERE source_memo_id = ? AND target_memo_id = ? AND relation_type = ?",
            params![source_id, target_id, relation_type],
        )?;
        Ok(())
    }

    /// 删除双向引用
    pub fn delete_bidirectional_reference(&self, memo_a: &str, memo_b: &str) -> CoreResult<()> {
        self.delete_relation(memo_a, memo_b, "REFERENCE")?;
        self.delete_relation(memo_b, memo_a, "REFERENCE")?;
        Ok(())
    }

    /// 获取笔记引用了哪些笔记（出向引用）
    pub fn get_outgoing_references(&self, memo_id: &str) -> CoreResult<Vec<String>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT target_memo_id FROM memo_relation
             WHERE source_memo_id = ? AND relation_type = 'REFERENCE'",
        )?;
        let rows: Vec<String> = stmt
            .query_map(params![memo_id], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 获取哪些笔记引用了当前笔记（入向引用）
    pub fn get_incoming_references(&self, memo_id: &str) -> CoreResult<Vec<String>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT source_memo_id FROM memo_relation
             WHERE target_memo_id = ? AND relation_type = 'REFERENCE'",
        )?;
        let rows: Vec<String> = stmt
            .query_map(params![memo_id], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 获取笔记的所有关联关系
    pub fn get_all_relations(&self, memo_id: &str) -> CoreResult<Vec<MemoRelation>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT source_memo_id, target_memo_id, relation_type, created_at
             FROM memo_relation
             WHERE source_memo_id = ? OR target_memo_id = ?
             ORDER BY created_at DESC",
        )?;
        let rows: Vec<MemoRelation> = stmt
            .query_map(params![memo_id, memo_id], |r| {
                Ok(MemoRelation {
                    source_memo_id: r.get(0)?,
                    target_memo_id: r.get(1)?,
                    relation_type: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }
}
