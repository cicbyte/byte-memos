<script setup lang="ts">
/**
 * AI 侧边栏（全局）：多轮流式对话面板
 *
 * - 挂载于 AppLayout 右侧滑出（fixed，不挤压内容布局）
 * - 助手消息走 MdPreview 渲染 Markdown；流式光标 + 思考折叠块
 * - 空态：sparkles 图标 + 快捷指令 chips（点击即发送）
 * - 会话持久化在 aiChat store（localStorage）
 */
import { nextTick, ref, watch } from "vue";
import { CloseOutlined, PlusOutlined, SendOutlined, StopOutlined, LoadingOutlined } from "@ant-design/icons-vue";
import { MdPreview } from "md-editor-v3";
import { useAiChatStore } from "@/stores/aiChat";
import { useAiProviderStore } from "@/stores/aiProvider";
import { useThemeStore } from "@/stores/theme";

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
  input.value = "";
  void chat.send(t);
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

function providerLabel(): string {
  const p = aiStore.enabledProvider();
  return p ? `${p.name} · ${p.model}` : "未启用供应商";
}
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
          <div class="ai-head-main">
            <span class="ai-title">AI 助手</span>
            <span class="ai-provider">{{ providerLabel() }}</span>
          </div>
          <button class="ai-head-btn" title="新对话" @click="chat.newChat()"><PlusOutlined /></button>
          <button class="ai-head-btn" title="收起" @click="chat.toggle(false)"><CloseOutlined /></button>
        </header>

        <!-- 消息流 -->
        <div ref="listEl" class="ai-list">
          <template v-if="chat.messages.length">
            <div v-for="(m, i) in chat.messages" :key="i" class="turn" :class="m.role">
              <div v-if="m.role === 'user'" class="bubble user">{{ m.content }}</div>
              <div v-else class="bubble assistant">
                <MdPreview
                  :model-value="m.content || '…'"
                  :theme="themeStore.isDark ? 'dark' : 'light'"
                  preview-theme="github"
                  code-theme="github"
                  :footers="[]"
                  class="ai-md"
                />
                <div v-if="m.tools?.length" class="tool-row">
                  <span v-for="(t, ti) in m.tools" :key="ti" class="tool-chip" :title="t">🔧 {{ t }}</span>
                </div>
                <span v-if="chat.running && i === chat.messages.length - 1" class="caret" />
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
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.turn {
  display: flex;
}
.turn.user {
  justify-content: flex-end;
}
.bubble {
  max-width: 88%;
  border-radius: 12px;
  font-size: 12.5px;
  line-height: 1.7;
}
.bubble.user {
  padding: 8px 12px;
  color: var(--hero-cta-text, #fff);
  background: linear-gradient(135deg, var(--color-primary), color-mix(in srgb, var(--color-primary) 82%, #000));
  border-bottom-right-radius: 4px;
  white-space: pre-wrap;
  word-break: break-word;
}
.bubble.assistant {
  padding: 2px 4px;
  color: var(--text-primary);
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-bottom-left-radius: 4px;
  min-width: 0;
}
/* MdPreview 尺寸收敛 */
.bubble.assistant :deep(.md-editor-preview-wrapper) {
  padding: 4px 8px;
}
.bubble.assistant :deep(.md-editor-preview) {
  font-size: 12.5px;
  line-height: 1.7;
  color: var(--text-primary);
}
.bubble.assistant :deep(.md-editor-preview p) {
  margin: 0 0 6px;
}
.bubble.assistant :deep(.md-editor-preview p:last-child) {
  margin-bottom: 0;
}
.bubble.assistant :deep(.md-editor-preview pre) {
  font-size: 11.5px;
}
.caret {
  display: inline-block;
  width: 7px;
  height: 13px;
  margin: 2px 0 0 3px;
  vertical-align: -2px;
  background: var(--color-primary);
  animation: caret-blink 0.9s steps(1) infinite;
}
@keyframes caret-blink {
  50% { opacity: 0; }
}
/* 工具调用 chips */
.tool-row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 4px 8px 6px;
  border-top: 1px dashed var(--border-light);
  margin-top: 4px;
}
.tool-chip {
  font-size: 10px;
  font-family: "JetBrains Mono", Consolas, monospace;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--color-primary) 25%, transparent);
  border-radius: 999px;
  padding: 1px 8px;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
