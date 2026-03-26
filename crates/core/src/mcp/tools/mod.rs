//! MCP 工具的业务实现（纯逻辑，返回 String，不依赖 rmcp 类型）
//!
//! 备忘录应用：search/list/get/stats/delete 五个只读+回收站工具（PRD F10 仅要求 MCP 可读）。
//! 各实体读取/搜索按域拆到子模块；统一入口（`unified_*`）与辅助函数分离。
//! `server.rs` 直接用的符号（`ToolCtx` / `unified_*` / `stats`）经 `pub use` 重导出。

use crate::ProjectService;

mod memos;
mod helpers;
mod unified;

pub use unified::{stats, unified_delete, unified_get, unified_list, unified_search};

/// 工具执行上下文：每个调用持有一个 service 句柄
pub struct ToolCtx {
    pub svc: ProjectService,
}

impl ToolCtx {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            svc: ProjectService::open_default()?,
        })
    }
}
