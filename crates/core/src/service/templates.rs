//! 笔记模板服务层。

use super::*;

impl super::ProjectService {
    /// 创建模板
    pub fn create_template(&self, name: &str, content: &str, description: &str) -> CoreResult<i64> {
        self.db.create_template(name, content, description)
    }

    /// 获取模板（按名称）
    pub fn get_template(&self, name: &str) -> CoreResult<Option<crate::db::templates::MemoTemplate>> {
        self.db.get_template(name)
    }

    /// 列出所有模板
    pub fn list_templates(&self) -> CoreResult<Vec<crate::db::templates::MemoTemplate>> {
        self.db.list_templates()
    }

    /// 更新模板
    pub fn update_template(&self, name: &str, content: Option<&str>, description: Option<&str>) -> CoreResult<()> {
        self.db.update_template(name, content, description)
    }

    /// 删除模板
    pub fn delete_template(&self, name: &str) -> CoreResult<()> {
        self.db.delete_template(name)
    }

    /// 从模板创建笔记
    pub fn create_memo_from_template(&self, template_name: &str, title: Option<&str>) -> CoreResult<Memo> {
        let template = self.db.get_template(template_name)?
            .ok_or_else(|| CoreError::NotFound(format!("模板「{template_name}」")))?;
        
        let input = AddMemoInput {
            title: title.unwrap_or(&template.name).to_string(),
            content: template.content,
            visibility: "PRIVATE".to_string(),
            category: None,
            tags: Vec::new(),
        };
        self.add_memo(&input)
    }

    /// 将笔记保存为模板
    pub fn save_memo_as_template(&self, short_id: &str, template_name: &str, description: &str) -> CoreResult<i64> {
        let memo = self.get_memo(short_id)?;
        self.create_template(template_name, &memo.content, description)
    }
}
