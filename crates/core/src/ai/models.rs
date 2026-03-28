//! 拉取供应商可用模型列表（参考 Cherry Studio「获取模型列表」）
//!
//! 各协议的 /models 端点与鉴权：
//! - openai-compatible：GET {base}/models，Bearer
//! - anthropic：GET {base}/v1/models，x-api-key + anthropic-version
//! - gemini：GET {base}/v1beta/models，x-goog-api-key（name 去掉 "models/" 前缀）
//! - ollama：GET {base}/api/tags（models[].name）

use anyhow::{anyhow, Context, Result};
use rig_core::providers::anthropic;
use serde_json::Value;

use super::factory::ProviderConfig;

const TIMEOUT_SECS: u64 = 15;

/// 拉取模型 id 列表（按 provider 协议分派；失败带上下文报错）
pub async fn fetch_model_list(cfg: &ProviderConfig) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .context("构造 HTTP 客户端失败")?;

    match cfg.provider_key_normalized() {
        "anthropic" => {
            let base = if cfg.base_url.trim().is_empty() {
                "https://api.anthropic.com".to_string()
            } else {
                anthropic::client::normalize_anthropic_base_url(&cfg.base_url)
            };
            let url = format!("{}/v1/models", base.trim_end_matches('/'));
            let resp = client
                .get(&url)
                .header("x-api-key", &cfg.api_key)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
                .context("请求 Anthropic /models 失败")?;
            ensure_ok(resp.status(), &url).await?;
            let v: Value = resp.json().await.context("解析 Anthropic 响应失败")?;
            Ok(str_list(v.get("data"), "id"))
        }
        "gemini" => {
            let base = if cfg.base_url.trim().is_empty() {
                "https://generativelanguage.googleapis.com".to_string()
            } else {
                cfg.base_url.trim().trim_end_matches('/').to_string()
            };
            let url = format!("{}/v1beta/models", base);
            let resp = client
                .get(&url)
                .header("x-goog-api-key", &cfg.api_key)
                .send()
                .await
                .context("请求 Gemini /models 失败")?;
            ensure_ok(resp.status(), &url).await?;
            let v: Value = resp.json().await.context("解析 Gemini 响应失败")?;
            Ok(str_list(v.get("models"), "name")
                .into_iter()
                .map(|n| n.strip_prefix("models/").unwrap_or(&n).to_string())
                .collect())
        }
        "ollama" => {
            let base = if cfg.base_url.trim().is_empty() {
                "http://localhost:11434".to_string()
            } else {
                cfg.base_url.trim().trim_end_matches('/').to_string()
            };
            let url = format!("{}/api/tags", base);
            let resp = client
                .get(&url)
                .send()
                .await
                .context("请求 Ollama /api/tags 失败")?;
            ensure_ok(resp.status(), &url).await?;
            let v: Value = resp.json().await.context("解析 Ollama 响应失败")?;
            Ok(str_list(v.get("models"), "name"))
        }
        _ => {
            let base = if cfg.base_url.trim().is_empty() {
                "https://api.openai.com/v1".to_string()
            } else {
                super::factory::normalize_openai_base_url(&cfg.base_url)
            };
            let url = format!("{}/models", base.trim_end_matches('/'));
            let mut req = client.get(&url);
            if !cfg.api_key.is_empty() {
                req = req.bearer_auth(&cfg.api_key);
            }
            let resp = req.send().await.context("请求 /models 失败")?;
            ensure_ok(resp.status(), &url).await?;
            let v: Value = resp.json().await.context("解析 /models 响应失败")?;
            Ok(str_list(v.get("data"), "id"))
        }
    }
}

async fn ensure_ok(status: reqwest::StatusCode, url: &str) -> Result<()> {
    if status.is_success() {
        return Ok(());
    }
    Err(anyhow!("HTTP {status}（{url}）；请检查端点与 API Key"))
}

/// 从 {"data"|"models": [..]} 中提取字符串字段，容忍缺失/异型
fn str_list(container: Option<&Value>, field: &str) -> Vec<String> {
    let Some(list) = container.and_then(|c| c.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|item| item.get(field).and_then(|v| v.as_str()))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_list_tolerates_shapes() {
        let v: Value = serde_json::json!({"data": [{"id": "a"}, {"id": 2}, {}, {"id": " b "}]});
        assert_eq!(str_list(v.get("data"), "id"), vec!["a".to_string(), "b".to_string()]);
        assert!(str_list(None, "id").is_empty());
        let v: Value = serde_json::json!({"models": [{"name": "models/gemini-x"}]});
        assert_eq!(str_list(v.get("models"), "name"), vec!["models/gemini-x".to_string()]);
    }
}
