# AI M1 交付（rig-core 对接 + 供应商管理 + 流式动作）

## 架构
- 引擎：rig-core 0.42.0（`default-features=false, features=["reqwest","native-tls"]`），core `ai` feature 门控
- 4 provider：`openai-compatible`（CompletionsClient，兼容 DeepSeek/Qwen/Moonshot/中转站）/ `anthropic` / `gemini` / `ollama`（本地无鉴权）
- base_url 归一化：trim/去尾斜杠/剥 `/chat/completions`/补 `/v1`（openai 系）；anthropic 用官方 `normalize_anthropic_base_url`
- 流式：`completion_request(...).stream()` → `StreamedAssistantContent`；Text→token、ReasoningDelta→thinking；取消 = CancellationToken `select!` + `stream.cancel()`

## 变更清单
- core：`db/schema/v18_ai_providers.rs`（+注册，沿用 legacy_collision 模式）、`db/ai.rs`（供应商 CRUD SQL，启用单选事务）、`ai/{mod,dpapi,factory,prompts,stream}.rs`、`service/ai.rs`（ProviderItem/Input/CheckResult + JSON 解析）、`Cargo.toml`（ai feature：rig-core/base64/futures/tokio-util/windows）
- desktop：`commands/ai.rs`（ai_provider_list/add/update/delete/check、ai_actions、ai_stream、ai_stream_cancel；锁内快照配置、锁外驱动流、Channel 推 token/thinking/done/error 信封）、`state.rs`（ai_cancels 注册表）、`lib.rs` 注册
- 前端：`api/ai.ts`（类型 + streamAi/StreamHandle/isCancelledError）、`stores/aiProvider.ts`（ensureReady 守卫→未配置跳设置）、`settings/components/AiProviderSection.vue`（设置页新增「AI 供应商」分区：列表/单选启用/增删改/连通测试，Key 永不回显）、`memos/aiActions.ts`（动作定义+宽松 JSON 解析+`# 标题行`提取）、`memos/components/AiResultModal.vue`（流式预览+停止+思考折叠+应用行）、卡片/时间轴 ops 均接入 ✨AI 菜单（8 动作 + 翻译三语子菜单）
- 应用动作：替换/追加/另存为新备忘录（文本）；标签/分类 chip 采纳（JSON）

## 安全与降级
- api_key DPAPI 加密（`dpapi:` 前缀）入库；接口只回 `api_key_set`；更新空串=保留原值
- 思考通道可选（无 reasoning 模型不推）；取消静默收尾不弹错误

## 验证
- `cargo check -p byte-memos-core --features ai` / `-p byte-memos-desktop` 全绿；`cargo test -p byte-memos-core --features ai` 49 通过（DPAPI 回环/JSON 解析/归一化/prompt 回落）
- `pnpm vue-tsc --noEmit` 全绿；tauri dev 已带 AI 层重启运行正常
