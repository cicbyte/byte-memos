//! 数据模型（对应 PRD §5.2 数据模型字段表）
//!
//! byte-memos 是笔记应用：保留 memos 实体与共享的 categories/tags；
//! 书签功能已移除。
//! 本模块通过 `pub use` 重导出全部类型，保持上游
//! `use crate::model::*` / `use crate::model::Xxx` 路径不变。

mod common;
mod config;
mod memo;
mod trash;

pub use common::{Category, FileNode, PagedResult, SearchHit, StatsOverview, TagStat, TagTreeNode};
pub use config::{ApiConfig, AppConfig, BackupConfig, PreferencesConfig};
pub use memo::{AddMemoInput, Memo, MemoFilter, MemoSearchHit, UpdateMemoInput};
pub use trash::{ItemType, TrashedItem, UnifiedSearchHit};

#[cfg(test)]
mod tests {
    use super::*;

    // ============ P3：枚举序列化往返 ============

    #[test]
    fn item_type_serde_lowercase() {
        // 唯一变体 Memo 序列化为 "memo"
        assert_eq!(serde_json::to_string(&ItemType::Memo).unwrap(), "\"memo\"");

        // 反向解析
        let b: ItemType = serde_json::from_str("\"memo\"").unwrap();
        assert_eq!(b, ItemType::Memo);
    }

    #[test]
    fn app_config_default_has_empty_keys() {
        let cfg = AppConfig::default();
        assert!(cfg.data_dir.is_none());
        assert!(cfg.backup.compress);
        assert!(cfg.api.key.is_empty());
        assert!(cfg.api.mcp_key.is_empty());
        assert!(cfg.api.allow_local_no_key);
    }

    #[test]
    fn trashed_item_serializes() {
        // 验证 TrashedItem 能正常序列化/反序列化
        let item = TrashedItem {
            short_id: "ABC12345".into(),
            item_type: ItemType::Memo,
            title: "已删除笔记".into(),
            description: "描述".into(),
            category: Some("work".into()),
            deleted_at: "2026-07-01T00:00:00Z".into(),
            created_at: "2026-06-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"item_type\":\"memo\""));
        let back: TrashedItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.short_id, "ABC12345");
        assert_eq!(back.item_type, ItemType::Memo);
    }
}
