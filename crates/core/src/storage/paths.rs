//! ProjectStorage 路径方法
//!
//! 知识库各类内容目录（projects/snippets/notes/diagrams/templates）已随知识库移除，
//! 仅保留数据根路径。

use std::path::Path;

use super::ProjectStorage;

impl ProjectStorage {
    /// 数据目录根（data/，内容文件与 db 在此之下）
    pub fn root(&self) -> &Path {
        &self.root
    }
}
