//! ProviderConfig 与 base_url 归一化
//!
//! rig-core 各 provider 的 base_url 语义不同：
//! - openai：API 根（含 /v1），后续拼 /chat/completions
//! - anthropic：站点根（内部自带归一化 `normalize_anthropic_base_url`）
//! - gemini：站点根（https://generativelanguage.googleapis.com）
//! - ollama：本地根（http://localhost:11434），无需 api key
//!
//! 用户输入的 base_url 统一在此归一化，兼容「复制了完整端点」的常见粘贴。

use serde::{Deserialize, Serialize};

/// 支持的 provider_key（未知值按 openai-compatible 处理）
pub const PROVIDER_KEYS: &[&str] = &["openai-compatible", "anthropic", "gemini", "ollama"];

/// 供应商运行配置（api_key 为解密后明文，仅 Rust 侧短暂持有）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: i64,
    pub temperature: f64,
    pub provider_key: String,
}

impl ProviderConfig {
    pub fn provider_key_normalized(&self) -> &str {
        match self.provider_key.as_str() {
            "anthropic" | "gemini" | "ollama" => self.provider_key.as_str(),
            _ => "openai-compatible",
        }
    }
}

/// openai-compatible base_url 归一化：去空白/尾斜杠、剥已拼好的
/// /chat/completions 端点。
/// - 裸域名（无路径）补 `/v1`（如 https://api.openai.com）
/// - 已带版本路径的（…/v1、GLM …/api/paas/v4）原样信任，不再追加
pub fn normalize_openai_base_url(base: &str) -> String {
    let mut s = base.trim().to_string();
    while s.ends_with('/') {
        s.pop();
    }
    let lower = s.to_ascii_lowercase();
    if lower.ends_with("/chat/completions") {
        s.truncate(s.len() - "/chat/completions".len());
        while s.ends_with('/') {
            s.pop();
        }
    }
    let has_path = s
        .split_once("://")
        .map(|(_, rest)| rest.contains('/'))
        .unwrap_or(false);
    if !has_path && !s.to_ascii_lowercase().ends_with("/v1") {
        s.push_str("/v1");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_base_url_forms_converge() {
        assert_eq!(
            normalize_openai_base_url("https://api.deepseek.com"),
            "https://api.deepseek.com/v1"
        );
        assert_eq!(
            normalize_openai_base_url("https://api.deepseek.com/v1"),
            "https://api.deepseek.com/v1"
        );
        assert_eq!(
            normalize_openai_base_url("https://x.com/v1/chat/completions/"),
            "https://x.com/v1"
        );
        // 带版本路径的供应商端点原样信任（GLM /v4 不追加 /v1）
        assert_eq!(
            normalize_openai_base_url("https://open.bigmodel.cn/api/paas/v4"),
            "https://open.bigmodel.cn/api/paas/v4"
        );
        assert_eq!(
            normalize_openai_base_url("https://api.xiaomimimo.com/v1/"),
            "https://api.xiaomimimo.com/v1"
        );
        assert_eq!(normalize_openai_base_url("  "), "/v1");
    }

    #[test]
    fn unknown_provider_falls_back() {
        let cfg = ProviderConfig {
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            max_tokens: 0,
            temperature: 0.0,
            provider_key: "weird".into(),
        };
        assert_eq!(cfg.provider_key_normalized(), "openai-compatible");
    }
}
