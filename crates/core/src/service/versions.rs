//! 笔记版本历史服务层。

use super::*;

/// 版本记录（带笔记标题）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoVersionInfo {
    /// 版本ID
    pub id: i64,
    /// 笔记ID
    pub memo_short_id: String,
    /// 笔记标题
    pub memo_title: String,
    /// 该版本的内容
    pub content: String,
    /// 创建时间
    pub created_at: String,
}

impl super::ProjectService {
    /// 创建版本快照（保存笔记时自动调用）
    pub fn create_version(&self, short_id: &str, content: &str) -> CoreResult<i64> {
        let memo_id = self.db.get_memo_int_id(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))?;
        self.db.create_version(memo_id, content)
    }

    /// 获取笔记的版本历史（按时间倒序）
    pub fn list_versions(&self, short_id: &str) -> CoreResult<Vec<MemoVersionInfo>> {
        let memo = self.get_memo(short_id)?;
        let memo_id = self.db.get_memo_int_id(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))?;
        let versions = self.db.list_versions(memo_id)?;
        Ok(versions.into_iter().map(|v| MemoVersionInfo {
            id: v.id,
            memo_short_id: short_id.to_string(),
            memo_title: memo.title.clone(),
            content: v.content,
            created_at: v.created_at,
        }).collect())
    }

    /// 回滚到指定版本（创建新版本记录，不删除历史）
    pub fn restore_version(&self, short_id: &str, version_id: i64) -> CoreResult<Memo> {
        // 获取版本内容
        let version = self.db.get_version(version_id)?
            .ok_or_else(|| CoreError::NotFound(format!("版本 {version_id}")))?;
        
        // 验证版本属于该笔记
        let memo_id = self.db.get_memo_int_id(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))?;
        if version.memo_id != memo_id {
            return Err(CoreError::Other(anyhow::anyhow!("版本 {version_id} 不属于笔记 {short_id}")));
        }

        // 更新笔记内容为版本内容
        let input = UpdateMemoInput {
            title: None,
            content: Some(version.content),
            visibility: None,
            category: None,
            tags: None,
            pinned: None,
        };
        self.update_memo(short_id, &input)
    }
}
