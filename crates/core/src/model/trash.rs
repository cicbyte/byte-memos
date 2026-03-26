//! 回收站与统一搜索数据类型

use serde::{Deserialize, Serialize};

/// 回收站条目类型（byte-memos 退化为笔记单一类型后仅 Memo）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    Memo,
}

impl ItemType {
    /// 序列化后的字符串形式（"memo"），与 DB 存储一致
    pub fn as_str(&self) -> &'static str {
        match self {
            ItemType::Memo => "memo",
        }
    }

    /// 从字符串反序列化（与 serde `rename_all=lowercase` 对齐）
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "memo" => Some(ItemType::Memo),
            _ => None,
        }
    }
}

/// 回收站统一条目（笔记软删除条目，用于回收站列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashedItem {
    pub short_id: String,
    pub item_type: ItemType,
    pub title: String,
    pub description: String,
    /// 原分类名（若有）
    #[serde(default)]
    pub category: Option<String>,
    /// 删除时间（ISO8601）
    pub deleted_at: String,
    /// 创建时间（ISO8601）
    pub created_at: String,
}

/// 统一搜索命中（笔记搜索结果，由 `search_all` 返回）
///
/// 中文/模糊命中走 LIKE 兜底（rank=0），英文/前缀命中走 FTS5（rank=bm25）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchHit {
    pub short_id: String,
    pub item_type: ItemType,
    pub title: String,
    /// 命中上下文（FTS 命中时为 snippet(...) 高亮片段；LIKE 兜底时为空串）
    pub snippet: String,
    #[serde(default)]
    pub category: Option<String>,
    /// 相关度（FTS bm25 负数越小越相关；LIKE 兜底统一 0.0）
    pub rank: f64,
}
