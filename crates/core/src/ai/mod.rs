//! AI 能力（feature = "ai"）
//!
//! rig-core 统一 provider 层：流式补全经 Tauri Channel 推前端。
//! - [`dpapi`]：api_key at-rest 加密（Windows DPAPI，非 Windows 透传）
//! - [`prompts`]：动作模板（整理/摘要/续写/扩写/润色/翻译/标签/分类/待办）
//! - [`factory`]：ProviderConfig → rig 客户端/模型构造与 base_url 归一化
//! - [`stream`]：流式引擎（增量回调 + CancellationToken 取消）
//!
//! 架构参考 byte-writer ai-integration（.reference/wiki/byte-writer/ai-integration.md），
//! provider 层按用户决策改用 rig-core（精确 pin，升级面收口在本模块三个文件）。

pub mod dpapi;
pub mod factory;
pub mod models;
pub mod prompts;
pub mod stream;
pub mod tools;

use serde::Serialize;

pub use factory::ProviderConfig;

/// 流式推送信封（Tauri Channel 序列化单元）
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamChunk {
    /// 正文增量
    Token { content: String },
    /// 思考通道增量（DeepSeek R 系 / Gemini thinking；无则不推）
    Thinking { content: String },
    /// 工具调用发起（侧边栏 agent 循环；前端展示「正在执行工具」）
    Tool { name: String, args: String },
    /// 工具执行结束（result 为宿主返回的 JSON 文本；前端展示详情/成败）
    ToolResult { name: String, result: String },
    /// 流结束（full = 聚合全文）
    Done { full: String },
    /// 流内错误（取消完成时不发 Error，静默收尾）
    Error { message: String },
}
