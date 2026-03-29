<script setup lang="ts">
/**
 * AI 侧边栏（全局）：多轮流式对话面板
 *
 * - 挂载于 AppLayout 右侧滑出（fixed，不挤压内容布局）
 * - 助手消息走 MdPreview 渲染 Markdown；流式光标 + 思考折叠块
 * - 空态：sparkles 图标 + 快捷指令 chips（点击即发送）
 * - 会话持久化在 aiChat store（localStorage）
 */
import { nextTick, ref, computed, watch } from "vue";
import { CloseOutlined, PlusOutlined, SendOutlined, StopOutlined, LoadingOutlined, DownOutlined, CopyOutlined } from "@ant-design/icons-vue";
import { message } from "ant-design-vue";
import { MdPreview } from "md-editor-v3";
import { useAiChatStore } from "@/stores/aiChat";
import { useAiProviderStore } from "@/stores/aiProvider";
import { useThemeStore } from "@/stores/theme";
import router from "@/router";
import type { ChatTurn } from "@/stores/aiChat";

const chat = useAiChatStore();
const aiStore = useAiProviderStore();
const themeStore = useThemeStore();

const input = ref("");
const listEl = ref<HTMLElement | null>(null);

const QUICK_PROMPTS = [
  "帮我想 5 个简洁的笔记标题风格示例",
  "把一段罗嗦的文字改写成待办清单，规则是什么？",
  "Markdown 常用语法速查（标题/列表/代码块）",
];

function send() {
  const t = input.value.trim();
  if (!t || chat.running) return;
  const p = aiStore.enabledProvider();
  if (!p?.model) {
    message.warning("当前供应商还未选择模型：点击顶部模型名即可选择");
    return;
  }
  input.value = "";
  void chat.send(t);
}

// ===== 模型选择（顶部下拉：用户手动获取/选择） =====
const provider = computed(() => aiStore.enabledProvider());
const providerName = computed(() => provider.value?.name ?? "未启用供应商");
const currentModel = computed(() => provider.value?.model ?? "");
const modelMenuItems = computed(() => {
  const p = provider.value;
  if (!p || !p.models.length) return [{ key: "__settings", label: "暂无模型 · 去设置获取" }];
  return p.models.map((m) => ({ key: m, label: m }));
});
async function onModelMenuClick({ key }: { key: string | number }) {
  if (key === "__settings") {
    chat.toggle(false);
    void router.push("/settings");
    return;
  }
  const p = provider.value;
  if (!p) return;
  const ok = await aiStore.selectModel(p.id, String(key));
  if (ok) message.success(`已切换模型：${key}`);
}

function onKey(e: KeyboardEvent) {
  if (e.isComposing || e.keyCode === 229) return;
  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
    e.preventDefault();
    send();
  }
}

function scrollBottom() {
  const el = listEl.value;
  if (el) el.scrollTop = el.scrollHeight;
}

/** 是否正在流式输出的那一条 */
function isStreaming(i: number): boolean {
  return chat.running && i === chat.messages.length - 1;
}

/** 渲染文本：流式时在末尾内联光标 */
function mdText(m: ChatTurn, i: number): string {
  const c = m.content;
  if (isStreaming(i)) return c ? c + " ▍" : "▍";
  return c || "（无内容）";
}

async function copyMsg(m: ChatTurn) {
  try {
    await navigator.clipboard.writeText(m.content);
    message.success("已复制");
  } catch {
    message.error("复制失败");
  }
}

watch(
  () => chat.messages.map((m) => m.content.length).join(",") + "|" + chat.messages.length,
  () => nextTick(scrollBottom),
);
watch(
  () => chat.open,
  (v) => {
    if (v) nextTick(scrollBottom);
  },
);
</script>

<template>
  <Teleport to="body">
    <Transition name="ai-side">
      <aside v-if="chat.open" class="ai-sidebar">
        <!-- 头部 -->
        <header class="ai-head">
          <span class="ai-badge">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z" />
              <path d="M20 3v4" /><path d="M22 5h-4" /><path d="M4 17v2" /><path d="M5 18H3" />
            </svg>
          </span>
          <a-dropdown trigger="['click']">
            <div class="ai-head-main" title="点击切换模型">
              <span class="ai-title">AI 助手</span>
              <span class="ai-provider" :class="{ 'ai-provider-none': !currentModel }">
                {{ providerName }} · {{ currentModel || "未选择模型" }} <DownOutlined class="ai-caret" />
              </span>
            </div>
            <template #overlay>
              <a-menu :selected-keys="currentModel ? [currentModel] : []" @click="onModelMenuClick">
                <a-menu-item v-for="m in modelMenuItems" :key="m.key">{{ m.label }}</a-menu-item>
              </a-menu>
            </template>
          </a-dropdown>
          <button class="ai-head-btn" title="新对话" @click="chat.newChat()"><PlusOutlined /></button>
          <button class="ai-head-btn" title="收起" @click="chat.toggle(false)"><CloseOutlined /></button>
        </header>

        <!-- 消息流 -->
        <div ref="listEl" class="ai-list">
          <template v-if="chat.messages.length">
            <div v-for="(m, i) in chat.messages" :key="i" class="turn" :class="m.role">
              <!-- 用户：右侧紧凑气泡 -->
              <div v-if="m.role === 'user'" class="bubble-user">{{ m.content }}</div>

              <!-- AI：无气泡沉浸式排版 -->
              <div
                v-else
                class="msg-ai"
                :class="{ streaming: isStreaming(i) }"
              >
                <span class="msg-avatar">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z" />
                  </svg>
                </span>

                <div class="msg-body">
                  <!-- 工具调用行 -->
                  <div v-if="m.tools?.length" class="msg-tools">
                    <span
                      v-for="(t, ti) in m.tools"
                      :key="ti"
                      class="tool-chip"
                      :class="{ pending: isStreaming(i) && ti === m.tools!.length - 1 }"
                      :title="t"
                    >
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
                      </svg>
                      {{ t }}
                    </span>
                  </div>

                  <MdPreview
                    :model-value="mdText(m, i)"
                    :theme="themeStore.isDark ? 'dark' : 'light'"
                    preview-theme="github"
                    code-theme="github"
                    :footers="[]"
                    class="ai-md"
                  />

                  <button class="msg-copy" title="复制全文" @click="copyMsg(m)">
                    <CopyOutlined />
                  </button>
                </div>
              </div>
            </div>
          </template>

          <!-- 空态：快捷指令 -->
          <div v-else class="ai-empty">
            <span class="ai-empty-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z" />
                <path d="M20 3v4" /><path d="M22 5h-4" /><path d="M4 17v2" /><path d="M5 18H3" />
              </svg>
            </span>
            <div class="ai-empty-title">和我聊聊</div>
            <div class="ai-empty-desc">随便问，或从下面的开始</div>
            <div class="ai-prompts">
              <button v-for="q in QUICK_PROMPTS" :key="q" class="ai-prompt-chip" @click="chat.send(q)">{{ q }}</button>
            </div>
          </div>
        </div>

        <!-- 输入区 -->
        <footer class="ai-input">
          <textarea
            v-model="input"
            rows="1"
            placeholder="问点什么…（Ctrl+Enter 发送）"
            @keydown="onKey"
            @input="($event.target as HTMLTextAreaElement).style.height = 'auto'; ($event.target as HTMLTextAreaElement).style.height = Math.min(($event.target as HTMLTextAreaElement).scrollHeight, 120) + 'px'"
          />
          <button v-if="chat.running" class="ai-send stop" title="停止" @click="chat.stop()">
            <StopOutlined />
          </button>
          <button v-else class="ai-send" :disabled="!input.trim()" title="发送" @click="send">
            <LoadingOutlined v-if="chat.running" /><SendOutlined v-else />
          </button>
        </footer>
      </aside>
    </Transition>
  </Teleport>
</template>

<style scoped>
.ai-sidebar {
  position: fixed;
  top: 8px;
  right: 10px;
  bottom: 10px;
  z-index: 90;
  display: flex;
  flex-direction: column;
  width: 400px;
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-panel, 14px);
  box-shadow: var(--shadow-panel-hover);
  overflow: hidden;
}

/* 滑入/滑出 */
.ai-side-enter-active,
.ai-side-leave-active {
  transition: transform 0.28s cubic-bezier(0.32, 0.72, 0.32, 1), opacity 0.28s ease;
}
.ai-side-enter-from,
.ai-side-leave-to {
  transform: translateX(calc(100% + 24px));
  opacity: 0;
}

/* ===== 头部 ===== */
.ai-head {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border-light);
  flex-shrink: 0;
}
.ai-badge {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  border-radius: 9px;
  flex-shrink: 0;
}
.ai-badge svg {
  width: 16px;
  height: 16px;
}
.ai-head-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
  cursor: pointer;
  border-radius: 8px;
  padding: 3px 6px;
  margin: -3px -6px;
  transition: background var(--transition-fast);
}
.ai-head-main:hover {
  background: color-mix(in srgb, var(--text-primary) 5%, transparent);
}
.ai-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}
.ai-provider {
  font-size: 10.5px;
  color: var(--text-tertiary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ai-provider-none {
  color: #d97706;
}
.ai-caret {
  font-size: 8px;
  margin-left: 2px;
}
.ai-head-btn {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  font-size: 12px;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: 7px;
  cursor: pointer;
  transition: all var(--transition-fast);
  flex-shrink: 0;
}
.ai-head-btn:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 7%, transparent);
}

/* ===== 消息流 ===== */
.ai-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 14px 14px 10px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.turn {
  display: flex;
}
.turn.user {
  justify-content: flex-end;
}
.turn + .turn.user {
  margin-top: 2px;
}

/* ---- 用户气泡：右侧渐变，紧凑 ---- */
.bubble-user {
  max-width: 84%;
  padding: 8px 13px;
  font-size: 12.5px;
  line-height: 1.65;
  color: var(--hero-cta-text, #fff);
  background: linear-gradient(135deg, var(--color-primary), color-mix(in srgb, var(--color-primary) 78%, #000));
  border-radius: 14px;
  border-bottom-right-radius: 5px;
  box-shadow: 0 1px 3px color-mix(in srgb, var(--color-primary) 25%, transparent);
  white-space: pre-wrap;
  word-break: break-word;
}

/* ---- AI 消息：无气泡沉浸式 ---- */
.msg-ai {
  position: relative;
  flex: 1;
  min-width: 0;
  padding-left: 30px;
}
.msg-avatar {
  position: absolute;
  left: 0;
  top: 1px;
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 9%, transparent);
  border-radius: 7px;
}
.msg-avatar svg {
  width: 12px;
  height: 12px;
}
.msg-body {
  position: relative;
  min-width: 0;
}

/* 工具调用行：内容上方轻量 chips */
.msg-tools {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-bottom: 7px;
}
.tool-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 10px;
  font-family: "JetBrains Mono", Consolas, monospace;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 7%, transparent);
  border: 1px solid color-mix(in srgb, var(--color-primary) 22%, transparent);
  border-radius: 6px;
  padding: 2px 8px;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tool-chip svg {
  width: 9px;
  height: 9px;
  flex-shrink: 0;
}
.tool-chip.pending {
  animation: chip-pulse 1.1s ease-in-out infinite;
}
@keyframes chip-pulse {
  50% { opacity: 0.45; }
}

/* Markdown 精修（窄侧栏排版） */
.msg-body :deep(.md-editor-preview-wrapper) {
  padding: 0;
}
.msg-body :deep(.md-editor-preview) {
  font-size: 12.5px;
  line-height: 1.7;
  color: var(--text-primary);
  word-break: break-word;
}
.msg-body :deep(.md-editor-preview p) {
  margin: 0 0 8px;
}
.msg-body :deep(.md-editor-preview p:last-child) {
  margin-bottom: 0;
}
.msg-body :deep(.md-editor-preview h1),
.msg-body :deep(.md-editor-preview h2),
.msg-body :deep(.md-editor-preview h3),
.msg-body :deep(.md-editor-preview h4) {
  margin: 14px 0 6px;
  font-size: 13px;
  font-weight: 650;
  line-height: 1.4;
}
.msg-body :deep(.md-editor-preview h1:first-child),
.msg-body :deep(.md-editor-preview h2:first-child),
.msg-body :deep(.md-editor-preview h3:first-child) {
  margin-top: 0;
}
.msg-body :deep(.md-editor-preview ul),
.msg-body :deep(.md-editor-preview ol) {
  margin: 0 0 8px;
  padding-left: 18px;
}
.msg-body :deep(.md-editor-preview li) {
  margin: 2px 0;
}
.msg-body :deep(.md-editor-preview blockquote) {
  margin: 8px 0;
  padding: 2px 10px;
  border-left: 2px solid color-mix(in srgb, var(--color-primary) 55%, transparent);
  background: color-mix(in srgb, var(--color-primary) 5%, transparent);
  border-radius: 0 6px 6px 0;
  color: var(--text-secondary);
}
.msg-body :deep(.md-editor-preview blockquote p) {
  margin: 4px 0;
}
.msg-body :deep(.md-editor-preview code:not(pre code)) {
  padding: 1px 5px;
  font-size: 11px;
  font-family: "JetBrains Mono", Consolas, monospace;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 8%, transparent);
  border-radius: 4px;
}
.msg-body :deep(.md-editor-preview pre) {
  margin: 8px 0;
  padding: 9px 11px;
  font-size: 11px;
  line-height: 1.6;
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: 8px;
  overflow-x: auto;
}
.msg-body :deep(.md-editor-preview pre code) {
  font-family: "JetBrains Mono", Consolas, monospace;
  background: transparent;
  padding: 0;
}
.msg-body :deep(.md-editor-preview table) {
  width: 100%;
  margin: 8px 0;
  font-size: 11.5px;
  border-collapse: collapse;
}
.msg-body :deep(.md-editor-preview th),
.msg-body :deep(.md-editor-preview td) {
  padding: 4px 8px;
  border: 1px solid var(--border-light);
  text-align: left;
}
.msg-body :deep(.md-editor-preview th) {
  background: var(--bg-input);
  font-weight: 600;
}
.msg-body :deep(.md-editor-preview a) {
  color: var(--color-primary);
  text-decoration: none;
  border-bottom: 1px solid color-mix(in srgb, var(--color-primary) 35%, transparent);
}
.msg-body :deep(.md-editor-preview hr) {
  margin: 10px 0;
  border: none;
  border-top: 1px solid var(--border-light);
}
.msg-body :deep(.md-editor-preview img) {
  max-width: 100%;
  border-radius: 8px;
}

/* 流式光标（内联 ▍ 字符渲染增强） */
.msg-body :deep(.md-editor-preview p:last-child),
.msg-body :deep(.md-editor-preview li:last-child) {
  /* 光标字符由 mdText 注入，这里不做特殊处理 */
}

/* hover 复制按钮 */
.msg-copy {
  position: absolute;
  top: -2px;
  right: 0;
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  font-size: 11px;
  color: var(--text-tertiary);
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: 6px;
  cursor: pointer;
  opacity: 0;
  transition: all var(--transition-fast);
}
.msg-ai:hover .msg-copy {
  opacity: 1;
}
.msg-copy:hover {
  color: var(--color-primary);
  border-color: color-mix(in srgb, var(--color-primary) 40%, transparent);
}

/* ===== 空态 ===== */
.ai-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  text-align: center;
  padding: 24px 16px;
}
.ai-empty-icon {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  color: var(--text-tertiary);
  background: color-mix(in srgb, var(--text-primary) 4%, transparent);
  border-radius: 50%;
  margin-bottom: 2px;
}
.ai-empty-icon svg {
  width: 22px;
  height: 22px;
}
.ai-empty-title {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-secondary);
}
.ai-empty-desc {
  font-size: 11px;
  color: var(--text-tertiary);
}
.ai-prompts {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 10px;
  width: 100%;
}
.ai-prompt-chip {
  padding: 8px 12px;
  font-size: 11.5px;
  text-align: left;
  font-family: inherit;
  color: var(--text-secondary);
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: 10px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.ai-prompt-chip:hover {
  color: var(--color-primary);
  border-color: color-mix(in srgb, var(--color-primary) 40%, transparent);
}

/* ===== 输入区 ===== */
.ai-input {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  padding: 10px 12px;
  border-top: 1px solid var(--border-light);
  flex-shrink: 0;
}
.ai-input textarea {
  flex: 1;
  min-height: 36px;
  max-height: 120px;
  padding: 8px 11px;
  font-size: 12.5px;
  font-family: inherit;
  line-height: 1.5;
  color: var(--text-primary);
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: 10px;
  outline: none;
  resize: none;
  transition: border-color var(--transition-fast);
}
.ai-input textarea:focus {
  border-color: color-mix(in srgb, var(--color-primary) 45%, transparent);
}
.ai-send {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  font-size: 13px;
  color: var(--hero-cta-text, #fff);
  background: linear-gradient(135deg, var(--color-primary), color-mix(in srgb, var(--color-primary) 82%, #000));
  border: none;
  border-radius: 10px;
  cursor: pointer;
  transition: all var(--transition-fast);
  flex-shrink: 0;
}
.ai-send:hover {
  filter: brightness(1.08);
}
.ai-send:disabled {
  opacity: 0.45;
  cursor: default;
  filter: none;
}
.ai-send.stop {
  color: #ef4444;
  background: color-mix(in srgb, #ef4444 10%, transparent);
}
</style>
