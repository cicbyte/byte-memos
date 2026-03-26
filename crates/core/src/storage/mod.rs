//! 文件存储层
//!
//! byte-memos 笔记应用：提供附件文件存储功能。
//! 附件存储在 `data/attachments/{memo_short_id}/` 目录下。

use std::path::PathBuf;

mod paths;

/// 文件存储根目录句柄（持有 `data/` 路径）
#[derive(Debug, Clone)]
pub struct ProjectStorage {
    pub(super) root: PathBuf,
}

impl ProjectStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}
