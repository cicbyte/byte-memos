//! 回收站：笔记软删条目的列出与清空。
//!
//! 笔记的软删（移入回收站）/ 恢复 / 彻底删除由 memos 域提供；
//! 本模块只负责跨类型的「列出 + 清空」聚合视图。
use super::*;

impl super::ProjectService {
    // ============ 回收站 ============

    /// 列出回收站全部条目（仅笔记软删除条目，按删除时间降序）
    pub fn list_trashed(&self) -> CoreResult<Vec<TrashedItem>> {
        let mut items = self.db.list_trashed_memos()?;
        // 合并后按删除时间降序
        items.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));
        Ok(items)
    }

    /// 清空回收站（对每个笔记条目执行彻底删除）
    pub fn empty_trash(&self) -> CoreResult<()> {
        let items = self.list_trashed()?;
        for item in items {
            let r = match item.item_type {
                ItemType::Memo => self.purge_memo(&item.short_id),
            };
            if let Err(e) = r {
                tracing::warn!("清空回收站时删除 {} 失败：{e}", item.short_id);
            }
        }
        Ok(())
    }
}
