//! 笔记数据模型

use serde::{Deserialize, Serialize};

/// 笔记实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memo {
    /// 短ID（8字符唯一标识）
    pub short_id: String,
    /// 标题
    pub title: String,
    /// Markdown 内容
    pub content: String,
    /// 可见性：PRIVATE / PROTECTED / PUBLIC
    pub visibility: String,
    /// 是否置顶
    pub pinned: bool,
    /// 分类ID
    pub category_id: Option<i64>,
    /// 分类名称（关联查询时填充）
    pub category: Option<String>,
    /// 标签列表
    pub tags: Vec<String>,
    /// 创建时间（ISO 8601）
    pub created_at: String,
    /// 更新时间（ISO 8601）
    pub updated_at: String,
    /// 删除时间（软删除）
    pub deleted_at: Option<String>,
}

/// 创建笔记输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemoInput {
    /// 标题
    pub title: String,
    /// Markdown 内容
    pub content: String,
    /// 可见性（默认 PRIVATE）
    #[serde(default = "default_visibility")]
    pub visibility: String,
    /// 分类
    pub category: Option<String>,
    /// 标签列表
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_visibility() -> String {
    "PRIVATE".to_string()
}

/// 更新笔记输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMemoInput {
    /// 标题（可选）
    pub title: Option<String>,
    /// 内容（可选）
    pub content: Option<String>,
    /// 可见性（可选）
    pub visibility: Option<String>,
    /// 分类（可选）
    pub category: Option<Option<String>>,
    /// 标签（可选，全量替换）
    pub tags: Option<Vec<String>>,
    /// 是否置顶（可选）
    pub pinned: Option<bool>,
}

/// 笔记过滤条件
#[derive(Debug, Clone, Default)]
pub struct MemoFilter {
    /// 按标签筛选
    pub tag: Option<String>,
    /// 按分类筛选
    pub category: Option<String>,
    /// 是否已归档
    pub archived: bool,
    /// 仅收藏
    pub favorited_only: bool,
    /// 分页：每页数量
    pub limit: Option<i64>,
    /// 分页：偏移量
    pub offset: Option<i64>,
}

/// 笔记搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoSearchHit {
    /// 短ID
    pub short_id: String,
    /// 标题
    pub title: String,
    /// 内容片段（高亮）
    pub snippet: Option<String>,
    /// 分类
    pub category: Option<String>,
    /// 标签
    pub tags: Vec<String>,
    /// 搜索排名
    pub rank: Option<f64>,
}
