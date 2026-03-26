//! 分类（categories）与标签（tags）管理。
use super::*;

impl super::ProjectService {
    // ============ 分类管理 ============

    pub fn list_categories(&self) -> CoreResult<Vec<Category>> {
        self.db.list_categories()
    }

    /// 新建分类（按名 upsert，重名则返回已有 id）
    pub fn create_category(&self, name: &str) -> CoreResult<i64> {
        self.db.upsert_category(name)
    }

    pub fn rename_category(&self, id: i64, new_name: &str) -> CoreResult<()> {
        self.db.rename_category(id, new_name)
    }

    pub fn delete_category(&self, id: i64) -> CoreResult<()> {
        self.db.delete_category(id)
    }

    // ============ 标签管理 ============

    /// 列出所有标签（树形结构）
    pub fn list_tags_tree(&self) -> CoreResult<Vec<TagTreeNode>> {
        self.db.list_tags_tree()
    }

    /// 设置标签颜色
    pub fn set_tag_color(&self, tag_name: &str, color: Option<&str>) -> CoreResult<()> {
        let conn = self.db.lock()?;
        conn.execute(
            "UPDATE tags SET color = ? WHERE name = ?",
            rusqlite::params![color, tag_name],
        )?;
        Ok(())
    }

    /// 删除标签（关联的笔记标签会被级联删除）
    pub fn delete_tag(&self, tag_name: &str) -> CoreResult<()> {
        let conn = self.db.lock()?;
        conn.execute(
            "DELETE FROM tags WHERE name = ?",
            rusqlite::params![tag_name],
        )?;
        Ok(())
    }
}
