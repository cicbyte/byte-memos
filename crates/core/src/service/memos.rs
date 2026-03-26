//! 备忘录（memos）：CRUD、列表、置顶、可见性、回收站、搜索。
//!
//! 笔记内容直接存 DB，FTS 由 db 层 `sync_memo_fts` 直接同步。visibility 可切换。

use super::*;

impl super::ProjectService {
    // ============ 备忘录 ============

    /// 新增备忘录：生成 shortId → 写库 → 关联标签 → 同步 FTS
    pub fn add_memo(&self, input: &AddMemoInput) -> CoreResult<Memo> {
        if input.title.trim().is_empty() && input.content.trim().is_empty() {
            return Err(CoreError::Other(anyhow::anyhow!("备忘录标题和内容不能同时为空")));
        }
        let short_id = self.generate_unique_memo_id()?;
        let category_id = match &input.category {
            Some(c) => Some(self.db.upsert_category(c)?),
            None => None,
        };
        let now = now_iso();
        let m = Memo {
            short_id: short_id.clone(),
            title: input.title.clone(),
            content: input.content.clone(),
            visibility: input.visibility.clone(),
            pinned: false,
            category_id,
            category: input.category.clone(),
            tags: input.tags.clone(),
            created_at: now.clone(),
            updated_at: now,
            deleted_at: None,
        };
        self.db.insert_memo(&m)?;
        let tag_ids: Vec<i64> = input
            .tags
            .iter()
            .map(|t| {
                // 支持层级标签：#工作/项目A → 父标签"工作"，子标签"项目A"
                if let Some((parent, child)) = t.split_once('/') {
                    self.db.upsert_tag_with_parent(child, Some(parent), None)
                } else {
                    self.db.upsert_tag(t)
                }
            })
            .collect::<CoreResult<Vec<_>>>()?;
        self.db.set_memo_tags(&short_id, &tag_ids)?;
        self.db.sync_memo_fts(&short_id)?;
        self.db
            .get_memo(&short_id)?
            .ok_or(CoreError::NotFound(short_id))
    }

    fn generate_unique_memo_id(&self) -> CoreResult<String> {
        for _ in 0..SHORT_ID_MAX_RETRIES {
            let id = shortid::generate();
            if !self.db.memo_id_exists(&id)? {
                return Ok(id);
            }
        }
        Err(CoreError::ShortIdCollision(SHORT_ID_MAX_RETRIES))
    }

    /// 按 shortId 取备忘录
    pub fn get_memo(&self, short_id: &str) -> CoreResult<Memo> {
        self.db
            .get_memo(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))
    }

    pub fn list_memos(&self, filter: &MemoFilter) -> CoreResult<Vec<Memo>> {
        self.db.list_memos(filter)
    }

    /// 分页列出备忘录（items + total）
    pub fn list_memos_paged(&self, filter: &MemoFilter) -> CoreResult<PagedResult<Memo>> {
        let items = self.db.list_memos(filter)?;
        let total = self.db.count_memos(filter)?;
        Ok(PagedResult { items, total })
    }

    /// 更新备忘录（覆盖式）：元数据 + 标签重置 + FTS 刷新 + 版本快照
    pub fn update_memo(&self, short_id: &str, input: &UpdateMemoInput) -> CoreResult<Memo> {
        let old = self
            .db
            .get_memo(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))?;
        
        // 如果内容有变化，创建版本快照
        if let Some(new_content) = &input.content {
            if new_content != &old.content {
                self.create_version(short_id, &old.content)?;
            }
        }
        
        let title = input.title.clone().unwrap_or(old.title);
        let content = input.content.clone().unwrap_or(old.content);
        let visibility = input.visibility.clone().unwrap_or(old.visibility);
        let pinned = input.pinned.unwrap_or(old.pinned);
        let category_id = match &input.category {
            Some(Some(c)) => Some(self.db.upsert_category(c)?),
            Some(None) => None,
            None => old.category_id,
        };
        self.db.update_memo_meta(
            short_id,
            &title,
            &content,
            &visibility,
            category_id,
            pinned,
        )?;
        if let Some(tags) = &input.tags {
            let tag_ids: Vec<i64> = tags
                .iter()
                .map(|t| {
                    // 支持层级标签：#工作/项目A → 父标签"工作"，子标签"项目A"
                    if let Some((parent, child)) = t.split_once('/') {
                        self.db.upsert_tag_with_parent(child, Some(parent), None)
                    } else {
                        self.db.upsert_tag(t)
                    }
                })
                .collect::<CoreResult<Vec<_>>>()?;
            self.db.set_memo_tags(short_id, &tag_ids)?;
        }
        self.db.sync_memo_fts(short_id)?;
        self.db
            .get_memo(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))
    }

    /// 移入回收站（软删除：打标记，标签关联保留）
    pub fn delete_memo(&self, short_id: &str) -> CoreResult<()> {
        self.db.trash_memo(short_id)?;
        Ok(())
    }

    /// 从回收站恢复（清标记 + 重建 FTS）
    pub fn restore_memo(&self, short_id: &str) -> CoreResult<()> {
        self.db.restore_memo(short_id)?;
        self.db.sync_memo_fts(short_id)?;
        Ok(())
    }

    /// 彻底删除备忘录（物理删除 DB 行 + 关联 CASCADE + FTS）
    pub fn purge_memo(&self, short_id: &str) -> CoreResult<()> {
        self.db.delete_memo(short_id)?;
        Ok(())
    }

    /// 切换置顶状态
    pub fn toggle_memo_pinned(&self, short_id: &str) -> CoreResult<Memo> {
        let cur = self
            .db
            .get_memo(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))?;
        self.db.set_memo_pinned(short_id, !cur.pinned)?;
        self.db
            .get_memo(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))
    }

    /// 设置可见性
    pub fn set_memo_visibility(&self, short_id: &str, visibility: &str) -> CoreResult<Memo> {
        let old = self
            .db
            .get_memo(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))?;
        self.db.update_memo_meta(
            short_id,
            &old.title,
            &old.content,
            visibility,
            old.category_id,
            old.pinned,
        )?;
        self.db
            .get_memo(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))
    }

    /// 活跃备忘录总数
    pub fn memo_count(&self) -> CoreResult<i64> {
        self.db.memo_count()
    }

    /// 备忘录全文搜索（FTS5 bm25 排名 + 高亮 snippet）
    pub fn search_memos(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> CoreResult<Vec<MemoSearchHit>> {
        self.db.search_memos(query, limit, offset)
    }
}
