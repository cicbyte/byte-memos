//! 笔记关联关系（双向引用）服务层。

use super::*;

/// 笔记关联详情（带笔记标题）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoRelationInfo {
    /// 关联的笔记ID
    pub short_id: String,
    /// 关联的笔记标题
    pub title: String,
    /// 关联类型
    pub relation_type: String,
}

impl super::ProjectService {
    /// 创建双向引用（A ↔ B）
    pub fn create_reference(&self, memo_a: &str, memo_b: &str) -> CoreResult<()> {
        // 验证两个笔记都存在
        self.get_memo(memo_a)?;
        self.get_memo(memo_b)?;
        // 防止自引用
        if memo_a == memo_b {
            return Err(CoreError::Other(anyhow::anyhow!("不能引用自身")));
        }
        self.db.create_bidirectional_reference(memo_a, memo_b)
    }

    /// 删除双向引用（A ↔ B）
    pub fn delete_reference(&self, memo_a: &str, memo_b: &str) -> CoreResult<()> {
        self.db.delete_bidirectional_reference(memo_a, memo_b)
    }

    /// 获取笔记的出向引用（当前笔记引用了哪些笔记）
    pub fn get_outgoing_references(&self, memo_id: &str) -> CoreResult<Vec<MemoRelationInfo>> {
        let ids = self.db.get_outgoing_references(memo_id)?;
        self.resolve_relation_infos(memo_id, &ids, "REFERENCE")
    }

    /// 获取笔记的入向引用（哪些笔记引用了当前笔记）
    pub fn get_incoming_references(&self, memo_id: &str) -> CoreResult<Vec<MemoRelationInfo>> {
        let ids = self.db.get_incoming_references(memo_id)?;
        self.resolve_relation_infos(memo_id, &ids, "REFERENCE")
    }

    /// 获取笔记的所有关联关系（去重）
    pub fn get_all_relations(&self, memo_id: &str) -> CoreResult<Vec<MemoRelationInfo>> {
        let relations = self.db.get_all_relations(memo_id)?;
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for rel in relations {
            let other_id = if rel.source_memo_id == memo_id {
                &rel.target_memo_id
            } else {
                &rel.source_memo_id
            };
            // 去重：同一笔记只显示一次
            if seen.contains(other_id) {
                continue;
            }
            seen.insert(other_id.clone());
            if let Ok(memo) = self.get_memo(other_id) {
                result.push(MemoRelationInfo {
                    short_id: memo.short_id,
                    title: memo.title,
                    relation_type: rel.relation_type,
                });
            }
        }
        Ok(result)
    }

    /// 辅助函数：将笔记ID列表解析为关联详情
    fn resolve_relation_infos(
        &self,
        _source_id: &str,
        target_ids: &[String],
        relation_type: &str,
    ) -> CoreResult<Vec<MemoRelationInfo>> {
        let mut result = Vec::new();
        for target_id in target_ids {
            if let Ok(memo) = self.get_memo(target_id) {
                result.push(MemoRelationInfo {
                    short_id: memo.short_id,
                    title: memo.title,
                    relation_type: relation_type.to_string(),
                });
            }
        }
        Ok(result)
    }
}
