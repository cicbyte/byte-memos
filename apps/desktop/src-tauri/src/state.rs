//! 应用状态：持有 core 的 ProjectService 单例 + MCP/API 服务运行状态

use std::sync::Mutex;
use byte_memos_core::ProjectService;

/// MCP HTTP 服务的运行状态
pub struct McpServerState {
    pub cancel: tokio_util::sync::CancellationToken,
    pub port: u16,
}

/// REST API 服务的运行状态
pub struct ApiServerState {
    pub cancel: tokio_util::sync::CancellationToken,
    pub port: u16,
}

/// 应用状态：持有 core 的 ProjectService 单例 + MCP/API 服务状态
pub struct AppState {
    pub svc: Mutex<ProjectService>,
    pub mcp: Mutex<Option<McpServerState>>,
    pub api: Mutex<Option<ApiServerState>>,
}

impl AppState {
    pub fn svc(&self) -> std::sync::MutexGuard<'_, ProjectService> {
        self.svc
            .lock()
            .expect("ProjectService mutex poisoned")
    }
}
