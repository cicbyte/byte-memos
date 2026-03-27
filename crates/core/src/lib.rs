//! byte-memos 核心库
//!
//! 业务逻辑 + 数据层 + MCP 工具实现的单一真相源。
//! 不依赖任何具体 transport（stdio/http）或 UI 框架，
//! CLI（apps/cli）和桌面端（apps/desktop/src-tauri）各自实现 transport/UI 层，
//! 共同复用本 crate 暴露的 [`service::ProjectService`]。

pub mod db;
pub mod error;
pub mod lang_ext;
pub mod model;
pub mod service;
pub mod shortid;
pub mod storage;

#[cfg(feature = "mcp")]
pub mod mcp;
#[cfg(feature = "ai")]
pub mod ai;

pub use error::{CoreError, CoreResult};
pub use model::*;
pub use service::{
    ProjectService, clear_data_dir, resolve_app_root, resolve_data_dir, restore_files,
    set_data_dir, set_exclude_dirs,
};
