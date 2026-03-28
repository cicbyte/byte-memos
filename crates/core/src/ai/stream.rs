//! 流式引擎：rig completion 流 → 增量回调
//!
//! - 单一入口 [`run_stream`]：按 provider_key 构造 rig 客户端，驱动流式补全
//! - 取消：tokio_util CancellationToken，取消时调 stream.cancel() 中断 HTTP，
//!   返回消息为 [`CANCELLED_MSG`]（调用方据此静默收尾，不再发 Error 信封）
//! - 思考通道（ReasoningDelta）可选：有则回调 on_delta("", thinking)，无则不推

use anyhow::{anyhow, Context, Result};
use rig_core::client::CompletionClient;
use rig_core::providers::{anthropic, gemini, ollama, openai};
use rig_core::streaming::StreamedAssistantContent;
use tokio_util::sync::CancellationToken;

use super::factory::ProviderConfig;

/// 取消哨兵：错误消息等于此值时调用方按「用户取消」处理
pub const CANCELLED_MSG: &str = "已取消";

/// 增量回调：(正文增量, 思考增量)。Send 约束保证驱动 Future 可跨线程（Tauri async command）。
pub type OnDelta<'a> = dyn FnMut(&str, &str) + Send + 'a;

/// 运行一次流式补全，返回聚合全文
pub async fn run_stream(
    cfg: &ProviderConfig,
    system: &str,
    user: &str,
    on_delta: &mut OnDelta<'_>,
    cancel: &CancellationToken,
) -> Result<String> {
    if cfg.model.trim().is_empty() {
        return Err(anyhow!("未配置模型名"));
    }
    match cfg.provider_key_normalized() {
        "anthropic" => {
            let base = if cfg.base_url.trim().is_empty() {
                None
            } else {
                Some(anthropic::client::normalize_anthropic_base_url(&cfg.base_url))
            };
            let mut b = anthropic::Client::builder().api_key(cfg.api_key.clone());
            if let Some(u) = base {
                b = b.base_url(u);
            }
            let client = b.build().map_err(|e| anyhow!("构造 Anthropic 客户端失败: {e}"))?;
            drive(client.completion_model(cfg.model.clone()), cfg, system, user, on_delta, cancel).await
        }
        "gemini" => {
            let mut b = gemini::Client::builder().api_key(cfg.api_key.clone());
            if !cfg.base_url.trim().is_empty() {
                b = b.base_url(cfg.base_url.trim());
            }
            let client = b.build().map_err(|e| anyhow!("构造 Gemini 客户端失败: {e}"))?;
            drive(client.completion_model(cfg.model.clone()), cfg, system, user, on_delta, cancel).await
        }
        "ollama" => {
            let mut b = ollama::Client::builder().api_key(String::new()); // ollama 无需鉴权
            if !cfg.base_url.trim().is_empty() {
                b = b.base_url(cfg.base_url.trim());
            }
            let client = b.build().map_err(|e| anyhow!("构造 Ollama 客户端失败: {e}"))?;
            drive(client.completion_model(cfg.model.clone()), cfg, system, user, on_delta, cancel).await
        }
        _ => {
            let mut b = openai::CompletionsClient::builder().api_key(cfg.api_key.clone());
            if !cfg.base_url.trim().is_empty() {
                b = b.base_url(super::factory::normalize_openai_base_url(&cfg.base_url));
            }
            let client = b
                .build()
                .map_err(|e| anyhow!("构造 OpenAI 兼容客户端失败: {e}"))?;
            drive(client.completion_model(cfg.model.clone()), cfg, system, user, on_delta, cancel).await
        }
    }
}

/// 泛型驱动：任意 rig CompletionModel 的请求构造 → 流消费 → 聚合
async fn drive<M>(
    model: M,
    cfg: &ProviderConfig,
    system: &str,
    user: &str,
    on_delta: &mut OnDelta<'_>,
    cancel: &CancellationToken,
) -> Result<String>
where
    M: rig_core::completion::CompletionModel + Clone,
{
    let mut req = model.completion_request(user.to_string());
    if !system.trim().is_empty() {
        req = req.preamble(system.to_string());
    }
    if cfg.temperature > 0.0 {
        req = req.temperature(cfg.temperature);
    }
    if cfg.max_tokens > 0 {
        req = req.max_tokens(cfg.max_tokens as u64);
    }
    let mut stream = req.stream().await.context("AI 流式请求失败")?;

    let mut full = String::new();
    loop {
        enum Step {
            Text(String),
            Thinking(String),
            Eof,
        }
        let step = tokio::select! {
            _ = cancel.cancelled() => {
                stream.cancel();
                return Err(anyhow!(CANCELLED_MSG));
            }
            item = futures::StreamExt::next(&mut stream) => match item {
                Some(Ok(StreamedAssistantContent::Text(t))) => Step::Text(t.text),
                Some(Ok(StreamedAssistantContent::ReasoningDelta { reasoning, .. })) => Step::Thinking(reasoning),
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(e).context("AI 流中断"),
                None => Step::Eof,
            },
        };
        match step {
            Step::Eof => break,
            Step::Text(t) => {
                if !t.is_empty() {
                    on_delta(&t, "");
                    full.push_str(&t);
                }
            }
            Step::Thinking(t) => {
                if !t.is_empty() {
                    on_delta("", &t);
                }
            }
        }
    }
    Ok(full)
}

/// 非流式便捷调用（连通测试 / 小型 JSON 任务）：内部仍走流，聚合后返回
pub async fn run_once(cfg: &ProviderConfig, system: &str, user: &str) -> Result<String> {
    let token = CancellationToken::new();
    let mut sink = |_: &str, _: &str| {};
    run_stream(cfg, system, user, &mut sink, &token).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::models::fetch_model_list;

    fn ollama_cfg(model: &str) -> ProviderConfig {
        ProviderConfig {
            provider_key: "ollama".into(),
            base_url: "http://localhost:11434".into(),
            api_key: String::new(),
            model: model.into(),
            max_tokens: 64,
            temperature: 0.2,
        }
    }

    /// 真实链路（需本机 Ollama）：cargo test -p byte-memos-core --features ai -- --ignored ollama_local
    #[tokio::test]
    #[ignore = "需要本机 Ollama（localhost:11434）运行"]
    async fn ollama_local_fetch_models() {
        let names = fetch_model_list(&ollama_cfg("")).await.expect("获取模型列表失败");
        assert!(!names.is_empty(), "本机 Ollama 无模型");
    }

    /// 真实流式往返：小模型（minicpm-v4.6:1b, 1.6GB）保证加载快
    #[tokio::test]
    #[ignore = "需要本机 Ollama 与 minicpm-v4.6:1b 模型"]
    async fn ollama_local_stream_roundtrip() {
        let cfg = ollama_cfg("openbmb/minicpm-v4.6:1b");
        let mut buf = String::new();
        let mut deltas = 0usize;
        let mut cb = |t: &str, _: &str| {
            deltas += 1;
            buf.push_str(t);
        };
        let cancel = CancellationToken::new();
        let full = run_stream(&cfg, "你是测试助手，只回复要求的内容。", "只回复两个字：正常", &mut cb, &cancel)
            .await
            .expect("流式请求失败");
        assert!(!full.trim().is_empty(), "无输出");
        assert!(deltas > 0, "未收到任何增量");
    }
}
