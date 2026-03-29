//! 流式引擎：rig completion 流 → 增量回调
//!
//! - 单一入口 [`run_stream`]：按 provider_key 构造 rig 客户端，驱动流式补全
//! - 取消：tokio_util CancellationToken，取消时调 stream.cancel() 中断 HTTP，
//!   返回消息为 [`CANCELLED_MSG`]（调用方据此静默收尾，不再发 Error 信封）
//! - 思考通道（ReasoningDelta）可选：有则回调 on_delta("", thinking)，无则不推

use anyhow::{anyhow, Context, Result};
use rig_core::client::CompletionClient;
use rig_core::completion::{CompletionModel, Message, ToolDefinition};
use rig_core::providers::{anthropic, gemini, ollama, openai};
use rig_core::streaming::StreamedAssistantContent;
use tokio_util::sync::CancellationToken;

use super::factory::ProviderConfig;

/// 取消哨兵：错误消息等于此值时调用方按「用户取消」处理
pub const CANCELLED_MSG: &str = "已取消";

/// 工具调用请求（agent 循环把模型发起的调用交给宿主执行）
#[derive(Debug, Clone)]
pub struct ToolExec {
    pub name: String,
    pub args_json: String,
}

/// 工具事件（agent 增量回调的可选项）：发起 / 结束
#[derive(Debug)]
pub enum ToolDeltaEvent<'a> {
    /// 模型发起调用（执行前）
    Start(&'a ToolExec),
    /// 宿主执行结束（result 为返回的 JSON 文本）
    End { name: &'a str, result: &'a str },
}

/// agent 事件回调：(正文增量, 思考增量, 工具事件可选项)
pub type AgentDelta<'a> = dyn FnMut(&str, &str, Option<&ToolDeltaEvent<'_>>) + Send + 'a;

/// agent 循环上限（单次对话内最多工具轮数，防失控）
const MAX_TOOL_ROUNDS: usize = 8;

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
    let mut wrapped = |t: &str, th: &str, _: Option<&ToolDeltaEvent<'_>>| on_delta(t, th);
    let mut noop = |_: &ToolExec| String::new();
    run_agent(cfg, system, &[], user, Vec::new(), "", &mut noop, &mut wrapped, cancel).await
}

/// 多轮流式对话：history 为 (是否用户, 内容) 顺序序列（不含本次 user 输入）
pub async fn run_chat_stream(
    cfg: &ProviderConfig,
    system: &str,
    history: &[(bool, String)],
    user: &str,
    on_delta: &mut OnDelta<'_>,
    cancel: &CancellationToken,
) -> Result<String> {
    let mut wrapped = |t: &str, th: &str, _: Option<&ToolDeltaEvent<'_>>| on_delta(t, th);
    let mut noop = |_: &ToolExec| String::new();
    run_agent(cfg, system, history, user, Vec::new(), "", &mut noop, &mut wrapped, cancel).await
}

/// 带内置工具的 agent 流（侧边栏对话）：模型可多轮发起工具调用，
/// `execute` 同步执行并返回结果 JSON 文本；文本增量实时回调。
/// thinking：思考等级（off/medium/high；off/high 对 ollama 走 think 参数）
pub async fn run_agent_stream(
    cfg: &ProviderConfig,
    system: &str,
    history: &[(bool, String)],
    user: &str,
    tools: Vec<ToolDefinition>,
    thinking: &str,
    execute: impl FnMut(&ToolExec) -> String,
    on_delta: &mut AgentDelta<'_>,
    cancel: &CancellationToken,
) -> Result<String> {
    run_agent(cfg, system, history, user, tools, thinking, execute, on_delta, cancel).await
}

async fn run_agent(
    cfg: &ProviderConfig,
    system: &str,
    history: &[(bool, String)],
    user: &str,
    tools: Vec<ToolDefinition>,
    thinking: &str,
    mut execute: impl FnMut(&ToolExec) -> String,
    on_delta: &mut AgentDelta<'_>,
    cancel: &CancellationToken,
) -> Result<String> {
    use rig_core::completion::message::{ToolCall, ToolResult, ToolResultContent, UserContent};

    if cfg.model.trim().is_empty() {
        return Err(anyhow!(
            "未配置默认模型：请到 设置 → AI 供应商 选择一个模型（或先点击「获取模型」）"
        ));
    }

    // 会话历史（跨工具轮累积：assistant toolcall 轮 + 工具结果）
    let mut chat: Vec<Message> = history
        .iter()
        .map(|(is_user, c)| {
            if *is_user {
                Message::user(c.clone())
            } else {
                Message::assistant(c.clone())
            }
        })
        .collect();
    let mut prompt_msg = Message::user(user.to_string());
    let mut full = String::new();

    macro_rules! with_model {
        ($model:expr) => {{
            let model = $model;
            for _round in 0..=MAX_TOOL_ROUNDS {
                let mut req = model.completion_request(prompt_msg.clone());
                if !system.trim().is_empty() {
                    req = req.preamble(system.to_string());
                }
                if !chat.is_empty() {
                    req = req.messages(chat.iter().cloned());
                }
                if !tools.is_empty() {
                    req = req.tools(tools.clone());
                }
                if cfg.temperature > 0.0 {
                    req = req.temperature(cfg.temperature);
                }
                if cfg.max_tokens > 0 {
                    req = req.max_tokens(cfg.max_tokens as u64);
                }
                // 思考等级（仅 ollama 协议支持 think 参数：off=false / high=high；
                // medium 不传用模型默认；其他供应商由调用方走系统提示词）
                if cfg.provider_key_normalized() == "ollama" {
                    match thinking {
                        "off" => {
                            req = req.additional_params(serde_json::json!({ "think": false }));
                        }
                        "high" => {
                            req = req.additional_params(serde_json::json!({ "think": "high" }));
                        }
                        _ => {}
                    }
                }
                let mut stream = req.stream().await.context("AI 流式请求失败")?;

                let mut round_text = String::new();
                let mut tool_calls: Vec<ToolCall> = Vec::new();
                loop {
                    enum Step {
                        Text(String),
                        Thinking(String),
                        Call(ToolCall),
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
                            Some(Ok(StreamedAssistantContent::ToolCall { tool_call, .. })) => Step::Call(tool_call),
                            Some(Ok(_)) => continue,
                            Some(Err(e)) => return Err(e).context("AI 流中断"),
                            None => Step::Eof,
                        },
                    };
                    match step {
                        Step::Eof => break,
                        Step::Text(t) => {
                            if !t.is_empty() {
                                on_delta(&t, "", None);
                                round_text.push_str(&t);
                                full.push_str(&t);
                            }
                        }
                        Step::Thinking(t) => {
                            if !t.is_empty() {
                                on_delta("", &t, None);
                            }
                        }
                        Step::Call(tc) => {
                            let exec = ToolExec {
                                name: tc.function.name.clone(),
                                args_json: tc.function.arguments.to_string(),
                            };
                            on_delta("", "", Some(&ToolDeltaEvent::Start(&exec)));
                            tool_calls.push(tc);
                        }
                    }
                }

                // 无工具调用 → 本轮即终轮
                if tool_calls.is_empty() {
                    return Ok(full);
                }

                // assistant 轮（文本 + 工具调用）入历史
                let mut content = Vec::new();
                if !round_text.trim().is_empty() {
                    content.push(rig_core::completion::AssistantContent::text(round_text));
                }
                for tc in &tool_calls {
                    content.push(rig_core::completion::AssistantContent::ToolCall(tc.clone()));
                }
                chat.push(Message::Assistant { id: None, content });

                // 执行工具 → 打包 tool_result 作为下一轮 prompt
                let mut results: Vec<UserContent> = Vec::new();
                for tc in &tool_calls {
                    let exec = ToolExec {
                        name: tc.function.name.clone(),
                        args_json: tc.function.arguments.to_string(),
                    };
                    let result_text = execute(&exec);
                    on_delta(
                        "",
                        "",
                        Some(&ToolDeltaEvent::End {
                            name: &exec.name,
                            result: &result_text,
                        }),
                    );
                    results.push(UserContent::ToolResult(ToolResult {
                        call: tc.id.clone(),
                        provider: None,
                        name: tc.function.name.clone(),
                        content: vec![ToolResultContent::text(result_text)],
                    }));
                }
                prompt_msg = Message::User { content: results };
            }
            Err(anyhow!("工具调用轮数超出上限（{MAX_TOOL_ROUNDS}）"))
        }};
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
            with_model!(client.completion_model(cfg.model.clone()))
        }
        "gemini" => {
            let mut b = gemini::Client::builder().api_key(cfg.api_key.clone());
            if !cfg.base_url.trim().is_empty() {
                b = b.base_url(cfg.base_url.trim());
            }
            let client = b.build().map_err(|e| anyhow!("构造 Gemini 客户端失败: {e}"))?;
            with_model!(client.completion_model(cfg.model.clone()))
        }
        "ollama" => {
            let mut b = ollama::Client::builder().api_key(String::new()); // ollama 无需鉴权
            if !cfg.base_url.trim().is_empty() {
                b = b.base_url(cfg.base_url.trim());
            }
            let client = b.build().map_err(|e| anyhow!("构造 Ollama 客户端失败: {e}"))?;
            with_model!(client.completion_model(cfg.model.clone()))
        }
        _ => {
            let mut b = openai::CompletionsClient::builder().api_key(cfg.api_key.clone());
            if !cfg.base_url.trim().is_empty() {
                b = b.base_url(super::factory::normalize_openai_base_url(&cfg.base_url));
            }
            let client = b
                .build()
                .map_err(|e| anyhow!("构造 OpenAI 兼容客户端失败: {e}"))?;
            with_model!(client.completion_model(cfg.model.clone()))
        }
    }
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

    /// 真实多轮对话：带历史（历史里藏答案，验证 history 真正进入请求）
    #[tokio::test]
    #[ignore = "需要本机 Ollama 与 minicpm-v4.6:1b 模型"]
    async fn ollama_local_chat_with_history() {
        let cfg = ollama_cfg("openbmb/minicpm-v4.6:1b");
        let history = vec![
            (true, "暗号是什么？请记住：暗号是芝麻开门。".to_string()),
            (false, "好的，我记住了。".to_string()),
        ];
        let cancel = CancellationToken::new();
        let mut sink = |_: &str, _: &str| {};
        let full = run_chat_stream(&cfg, "你是测试助手。", &history, "暗号是什么？只回复暗号本身。", &mut sink, &cancel)
            .await
            .expect("多轮流式失败");
        assert!(!full.trim().is_empty(), "无输出");
    }

    /// 真实工具调用：qwen3.5 支持 tools；验证 ToolCall 事件到达且执行器被调、结果回填后收尾
    #[tokio::test]
    #[ignore = "需要本机 Ollama 与 qwen3.5:4b 模型（工具调用）"]
    async fn ollama_local_tool_call_roundtrip() {
        use rig_core::completion::ToolDefinition;
        let cfg = ollama_cfg("qwen3.5:4b");
        let tools = vec![ToolDefinition {
            name: "get_time".into(),
            description: "获取当前时间。需要知道现在几号/几点时调用。".into(),
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        }];
        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called2 = called.clone();
        let execute = move |exec: &ToolExec| {
            assert_eq!(exec.name, "get_time");
            called2.store(true, std::sync::atomic::Ordering::SeqCst);
            r#"{"ok":true,"now":"2026-02-06 12:00:00"}"#.to_string()
        };
        let mut tool_seen = false;
        let mut cb = |_: &str, _: &str, tool: Option<&ToolDeltaEvent<'_>>| {
            if matches!(tool, Some(ToolDeltaEvent::Start(_))) {
                tool_seen = true;
            }
        };
        let cancel = CancellationToken::new();
        let full = run_agent_stream(
            &cfg,
            "你是测试助手。",
            &[],
            "现在几点了？请用工具查询后回答。",
            tools,
            "",
            execute,
            &mut cb,
            &cancel,
        )
        .await
        .expect("工具调用流失败");
        assert!(tool_seen, "未收到 ToolCall 事件");
        assert!(called.load(std::sync::atomic::Ordering::SeqCst), "执行器未被调用");
        assert!(!full.trim().is_empty(), "工具轮后无最终文本");
    }

    /// 全链路复现（侧边栏场景）：真实内置工具 + 同款系统提示词 + create_memo 类指令，
    /// 打印事件序列（TOOL/TEXT/THINK）与最终全文，验证 Tool 事件与收尾文本。
    /// 注意 max_tokens 需给足（qwen3.5 思考 token 计入预算，64 会被思考烧光）
    #[tokio::test]
    #[ignore = "需要本机 Ollama 与 qwen3.5:4b 模型（内置工具全链路）"]
    async fn ollama_local_builtin_tools_poem() {
        let cfg = ProviderConfig {
            provider_key: "ollama".into(),
            base_url: "http://localhost:11434".into(),
            api_key: String::new(),
            model: "qwen3.5:4b".into(),
            max_tokens: 1024,
            temperature: 0.2,
        };
        let tools = crate::ai::tools::definitions();
        let events: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        // 不落库：模拟执行成功，隔离 UI 问题
        let mut tool_calls = 0usize;
        let execute = move |_exec: &ToolExec| {
            tool_calls += 1;
            format!("{{\"ok\":true,\"short_id\":\"test{}\",\"title\":\"诗\"}}", tool_calls)
        };
        let ev2 = events.clone();
        let mut cb = move |t: &str, th: &str, tool: Option<&ToolDeltaEvent<'_>>| {
            let mut e = ev2.lock().unwrap();
            if let Some(ToolDeltaEvent::Start(x)) = tool {
                e.push(format!("TOOL {} args={}", x.name, x.args_json));
            } else if let Some(ToolDeltaEvent::End { name, result }) = tool {
                e.push(format!("TOOL_END {} result={}", name, result));
            } else if !th.is_empty() {
                let n = e.last_mut().filter(|s| s.starts_with("THINK")).map(|s| s.len()).unwrap_or(0);
                if n == 0 {
                    e.push(format!("THINK({})", th.len()));
                } else if let Some(s) = e.last_mut() {
                    *s = format!("THINK({})", s.len() - 6 + th.len());
                }
            } else if !t.is_empty() {
                e.push(format!("TEXT[{}]", t));
            }
        };
        let system = "你是 byte-memos 备忘录应用内置的 AI 助手。用简体中文简洁回答，可使用 Markdown 排版。\
                      你有内置工具可以查询与操作用户的备忘录（搜索、列出、读取、创建、更新、删除、分类/标签/统计），\
                      涉及用户数据的问题优先调用工具获取真实数据，不要编造；用户明确要求记录/保存时用 create_memo。";
        let cancel = CancellationToken::new();
        let full = run_agent_stream(&cfg, system, &[], "添加一首随机李白的诗", tools, "off", execute, &mut cb, &cancel)
            .await
            .expect("全链路失败");
        let e = events.lock().unwrap();
        eprintln!("== EVENTS({}) ==", e.len());
        for x in e.iter() {
            eprintln!("  {x}");
        }
        eprintln!("== FULL = {full:?}");
        assert!(
            e.iter().any(|x| x.starts_with("TOOL ")),
            "未收到 TOOL 事件（模型未调用工具）"
        );
        assert!(
            e.iter().any(|x| x.starts_with("TOOL_END")),
            "未收到 TOOL_END 事件（执行结果未回传）"
        );
    }
}
