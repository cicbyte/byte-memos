//! MCP server：rmcp 工具定义 + 双 transport 启动
//!
//! 工具实现委托给 [`crate::mcp::tools`]；本模块只负责 rmcp 胶水与传输层。
//! 备忘录应用：search / list / get / stats / delete 五个只读 + 回收站工具（PRD F10）。

use std::sync::Arc;

use anyhow::Result;
use rmcp::{
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::*,
    schemars, tool, tool_handler, tool_router,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;

use super::tools::{self, ToolCtx};

/// ============ 工具入参 ============

/// search 工具参数
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchInput {
    /// 搜索关键词
    pub query: String,
    /// 搜索类型：memo / all（默认 all，跨类型合并搜索，支持中文）
    #[serde(default = "default_search_type")]
    pub r#type: String,
    /// 结果上限
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// 偏移量（翻页用，默认 0）
    #[serde(default)]
    pub offset: i64,
}

fn default_search_type() -> String {
    "all".into()
}

fn default_limit() -> i64 {
    20
}

/// list 工具参数
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListInput {
    /// 列出哪种内容：memo / tag / category
    pub r#type: String,
    /// 数量上限
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// 偏移量（翻页用）
    #[serde(default)]
    pub offset: i64,
}

/// get 工具参数
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetInput {
    /// 备忘录 shortId
    pub short_id: String,
    /// 内容类型（当前仅 memo）
    pub r#type: String,
}

/// delete 工具参数
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteInput {
    /// 备忘录 shortId
    pub short_id: String,
    /// 删除类型（当前仅 memo）
    pub r#type: String,
}

/// MCP 服务器：每个会话持有一个 ToolCtx
#[derive(Clone)]
pub struct ByteLinkMcpServer {
    ctx: Arc<ToolCtx>,
    // tool_router 字段由 #[tool_handler] 宏在生成的 ServerHandler 实现里读取，
    // 编译器静态分析看不到该引用，故显式抑制 dead_code。
    #[allow(dead_code)]
    tool_router: ToolRouter<ByteLinkMcpServer>,
}

impl ByteLinkMcpServer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            ctx: Arc::new(ToolCtx::new()?),
            tool_router: Self::tool_router(),
        })
    }
}

#[tool_router]
impl ByteLinkMcpServer {
    /// 搜索备忘录（统一入口）
    #[tool(description = "搜索备忘录。type: memo/all（默认 all 跨类型合并，支持中文）。\
                          返回命中列表（标题 + ID）。limit/offset 分页。")]
    async fn search(
        &self,
        Parameters(SearchInput { query, r#type, limit, offset }): Parameters<SearchInput>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(tools::unified_search(
            &self.ctx, &query, &r#type, limit, offset,
        )))
    }

    /// 列出备忘录/标签/分类（统一入口）
    #[tool(description = "列出内容。type: memo/tag/category。\
                          tag 和 category 不支持 offset。limit 默认 20。")]
    async fn list(
        &self,
        Parameters(ListInput { r#type, limit, offset }): Parameters<ListInput>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(tools::unified_list(&self.ctx, &r#type, limit, offset)))
    }

    /// 读取备忘录详情
    #[tool(description = "读取备忘录详情（标题/内容/标签/分类/置顶/可见性）。type: memo。")]
    async fn get(
        &self,
        Parameters(GetInput { short_id, r#type }): Parameters<GetInput>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(tools::unified_get(&self.ctx, &short_id, &r#type)))
    }

    /// 统计概览
    #[tool(description = "返回统计概览：备忘录/标签/分类数量与存储占用。")]
    async fn stats(&self) -> Result<CallToolResult, McpError> {
        Ok(text_result(tools::stats(&self.ctx)))
    }

    /// 删除备忘录（移入回收站，可恢复）
    #[tool(description = "删除备忘录（移入回收站，可恢复）。type: memo。")]
    async fn delete(
        &self,
        Parameters(DeleteInput { short_id, r#type }): Parameters<DeleteInput>,
    ) -> Result<CallToolResult, McpError> {
        Ok(text_result(tools::unified_delete(&self.ctx, &short_id, &r#type)))
    }
}

/// ServerHandler：声明能力并挂上工具路由
#[tool_handler]
impl ServerHandler for ByteLinkMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::from_build_env())
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_instructions(
            "byte-link 本地备忘录库。5 个工具：\n\
             search(query, type, limit, offset) — 搜索，type: memo/all\n\
             list(type, limit, offset) — 列出，type: memo/tag/category\n\
             get(short_id, type) — 读取备忘录详情\n\
             stats() — 统计概览\n\
             delete(short_id, type) — 删除（移入回收站，可恢复）"
                .to_string(),
        )
    }
}

fn text_result(text: String) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(text)])
}

// ============ transport ============

/// stdio 模式
pub async fn serve_stdio() -> Result<()> {
    let server = ByteLinkMcpServer::new()?;
    tracing::info!("MCP stdio server starting");
    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| anyhow::anyhow!("stdio serve 失败：{e:?}"))?;
    service.waiting().await?;
    Ok(())
}

/// http 模式（StreamableHTTP via axum）。api_key 非空则启用认证。
pub async fn serve_http(host: &str, port: u16, api_key: &str) -> Result<()> {
    let ct = tokio_util::sync::CancellationToken::new();
    let handle = serve_http_managed(host, port, api_key.to_string(), ct.clone())?;
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        ct.cancel();
    });
    handle.await?;
    Ok(())
}

/// 启动 HTTP MCP 服务，返回 JoinHandle。
///
/// `api_key` 非空时启用认证中间件（X-API-Key / Bearer token）。
/// 桌面端用此函数：spawn 到后台，存 CancellationToken 用于外部停止。
pub fn serve_http_managed(
    host: &str,
    port: u16,
    api_key: String,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<tokio::task::JoinHandle<()>> {
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    };

    let service = StreamableHttpService::new(
        || {
            ByteLinkMcpServer::new()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default().with_cancellation_token(cancel),
    );

    let mut app = axum::Router::new().nest_service("/mcp", service);

    // 可选 API Key 认证
    if !api_key.is_empty() {
        app = app.layer(axum::middleware::from_fn_with_state(
            api_key,
            mcp_auth_middleware,
        ));
        tracing::info!("MCP HTTP server enabled API Key auth");
    }

    let addr = format!("{host}:{port}");
    let rt = tokio::runtime::Handle::current();
    let handle = rt.spawn(async move {
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                tracing::info!("MCP HTTP server listening on http://{}/mcp", addr);
                let _ = axum::serve(listener, app).await;
            }
            Err(e) => tracing::error!("MCP HTTP bind {addr} 失败：{e}"),
        }
    });
    Ok(handle)
}

/// MCP HTTP API Key 认证中间件（可选启用）
async fn mcp_auth_middleware(
    axum::extract::State(expected_key): axum::extract::State<String>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    // 检查 X-API-Key 头
    let provided = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided == expected_key {
        return next.run(req).await;
    }
    // 检查 Bearer token
    let auth = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if let Some(token) = auth.strip_prefix("Bearer ") {
        if token == expected_key {
            return next.run(req).await;
        }
    }
    use axum::response::IntoResponse;
    (
        axum::http::StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({"error": "Invalid or missing API key"})),
    )
        .into_response()
}
