//! 文件存储层
//!
//! byte-link 退化为书签单一类型后：内容文件仅余 favicon 本地缓存（`data/favicons/`）。
//! 项目/片段/笔记/图表/模板/资产的目录与 CRUD 已随知识库移除。
//!
//! 各类路径与 CRUD 按域拆到子模块，均以 `impl ProjectStorage` 扩展。

use std::path::PathBuf;

mod favicons;
mod helpers;
mod paths;

/// 书签文件存储根目录句柄（持有 `data/` 路径）
#[derive(Debug, Clone)]
pub struct ProjectStorage {
    pub(super) root: PathBuf,
}

impl ProjectStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[cfg(test)]
mod tests;
