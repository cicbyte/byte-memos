//! 书签数据类型（byte-link 的核心领域实体）
//!
//! 与 projects/snippets/notes/templates/diagrams 同为内容载体，但面向「书签」：
//! - short_id 为主键（与全库一致，复用 trash / FTS / 标签 / 回收站机制）
//! - url 为业务唯一键：活跃书签（deleted_at IS NULL）内唯一（部分唯一索引），
//!   软删除后 url 可被重新添加
//! - description 为短文本，直接存 DB（不像 code/note 落盘到文件），
//!   故 FTS 用 `INSERT ... SELECT` 直接同步 DB 列，无需 service 读文件
//! - is_archived 归档（书签特有：不常用但保留，区别于 deleted_at 回收站）

use serde::{Deserialize, Serialize};

/// 书签主记录（bookmarks 表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub short_id: String,
    pub title: String,
    /// 书签地址（业务唯一键；活跃行内唯一）
    pub url: String,
    pub description: String,
    pub category_id: Option<i64>,
    pub category: Option<String>,
    /// 本地缓存的 favicon 文件相对路径（M5 抓取；空串表示未缓存）
    #[serde(default)]
    pub favicon_path: String,
    /// 访问次数（每次「打开浏览器」+1）
    #[serde(default)]
    pub visit_count: i64,
    /// 是否归档（书签特有：不常用但保留，区别于回收站）
    #[serde(default)]
    pub is_archived: bool,
    pub is_favorited: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 软删除时间（None=正常，Some=已进回收站）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

/// 新增书签的输入
#[derive(Debug, Clone, Default)]
pub struct AddBookmarkInput {
    /// url 必填
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    /// 初始 favicon 路径（M5 由抓取逻辑填入）
    pub favicon_path: Option<String>,
}

/// 更新书签的输入（字段为 None 表示保留原值；tags 为空 Vec 表示清空标签）
#[derive(Debug, Clone, Default)]
pub struct UpdateBookmarkInput {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub favicon_path: Option<String>,
    pub is_archived: Option<bool>,
}

/// 书签列表过滤条件
#[derive(Debug, Clone, Default)]
pub struct BookmarkFilter {
    pub tag: Option<String>,
    pub category: Option<String>,
    /// 仅看已归档 / 仅看未归档；None=全部
    pub archived: Option<bool>,
    pub favorited_only: bool,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 书签搜索命中（F2 全文搜索返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkSearchHit {
    pub short_id: String,
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub rank: f64,
}
