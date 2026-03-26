//! 通用数据类型：分页容器、文件树、搜索命中、统计、分类、标签统计

use serde::{Deserialize, Serialize};

/// 分页结果容器（`list_*_paged` 系列方法/命令返回）
///
/// `items` 为当前页数据，`total` 为满足过滤条件的总数（用于前端无限滚动 footer 展示）。
#[derive(Debug, Clone, Serialize)]
pub struct PagedResult<T> {
    pub items: Vec<T>,
    pub total: i64,
}

/// 文件树节点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub struct FileNode {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileNode>>,
    pub size: u64,
}

/// 搜索结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub short_id: String,
    pub title: String,
    pub snippet: String,
    pub rank: f64,
    pub matched_tags: Vec<String>,
}

/// 统计概览（dashboard 用）
///
/// byte-memos 笔记应用：仅保留笔记计数与共享的标签/分类/存储/置顶/回收站计数。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsOverview {
    /// 活跃笔记数
    #[serde(default)]
    pub memo_count: i64,
    pub tag_count: i64,
    pub category_count: i64,
    pub total_size: u64,
    pub pinned_count: i64,
    /// 回收站内条目数（软删除的笔记）
    #[serde(default)]
    pub trashed_count: i64,
}

/// 分类（含该分类下的笔记数）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    /// 分类主题色（hex，可空——未设置时前端用默认色）
    #[serde(default)]
    pub color: Option<String>,
    /// 该分类下的活跃笔记数
    #[serde(default)]
    pub memo_count: i64,
}

/// 标签使用统计项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagStat {
    pub tag: String,
    pub count: i64,
}

/// 标签树节点（支持层级结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagTreeNode {
    /// 标签ID
    pub id: i64,
    /// 标签名
    pub name: String,
    /// 父标签ID（None 表示顶层标签）
    pub parent_id: Option<i64>,
    /// 标签颜色（可选）
    pub color: Option<String>,
    /// 关联的笔记数量
    pub count: i64,
}
