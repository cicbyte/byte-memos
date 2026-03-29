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
import { CloseOutlined, PlusOutlined, SendOutlined, StopOutlined, LoadingOutlined, DownOutlined, CopyOutlined, RightOutlined } from "@ant-design/icons-vue";
import { message } from "ant-design-vue";
import MdStreamPreview from "@/components/common/MdStreamPreview.vue";
import { describeStep } from "./aiToolSteps";
import { useAiChatStore } from "@/stores/aiChat";
import { useAiProviderStore } from "@/stores/aiProvider";
import { useThemeStore } from "@/stores/theme";
import router from "@/router";
import type { ChatTurn, ToolStep } from "@/stores/aiChat";

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
    message.warning("当前供应商还未选择模型：点击底部模型名即可选择");
    return;
  }
  input.value = "";
  void chat.send(t);
}

// ===== 模型选择（底部工具条下拉：用户手动获取/选择） =====
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

// ===== 思考等级（底部工具条下拉） =====
const THINKING_LABELS: Record<string, string> = { off: "关闭", medium: "适中", high: "深入" };
const thinkingLabel = computed(() => THINKING_LABELS[chat.thinking] ?? "适中");
function onThinkingClick({ key }: { key: string | number }) {
  chat.setThinking(String(key));
}

function onKey(e: KeyboardEvent) {
  if (e.isComposing || e.keyCode === 229) return;
  if (e.key !== "Enter") return;
  // Alt+Enter = 换行；Enter = 发送；其余组合键保持默认
  if (e.altKey) {
    e.preventDefault();
    const ta = e.target as HTMLTextAreaElement;
    ta.setRangeText("\n", ta.selectionStart, ta.selectionEnd, "end");
    input.value = ta.value;
    nextTick(() => {
      ta.style.height = "auto";
      ta.style.height = Math.min(ta.scrollHeight, 120) + "px";
    });
    return;
  }
  if (e.ctrlKey || e.metaKey || e.shiftKey) return;
  e.preventDefault();
  send();
}

function scrollBottom() {
  const el = listEl.value;
  if (el) el.scrollTop = el.scrollHeight;
}

/** 是否正在流式输出的那一条 */
function isStreaming(i: number): boolean {
  return chat.running && i === chat.messages.length - 1;
}

/** 渲染文本：流式时在末尾内联光标；空回复兜底（思考型模型可能只思考不答） */
function mdText(m: ChatTurn, i: number): string {
  const c = m.content;
  if (isStreaming(i)) return c ? c + " ▍" : "▍";
  if (c) return c;
  if (m.steps?.length) return "（已执行操作，模型未返回文字说明）";
  return "（无内容）";
}

function copyMsg(m: ChatTurn) {
  try {
    void navigator.clipboard.writeText(m.content);
    message.success("已复制");
  } catch {
    message.error("复制失败");
  }
}

// ===== 工具步骤折叠组（ZCode ToolLayout 模式：运行中自动展开、完成自动收起） =====
const stepsExpanded = ref<Record<number, boolean>>({});

function hasRunning(m: ChatTurn): boolean {
  return !!m.steps?.some((s) => s.status === "running");
}

/** 组是否展开：流式中且有 running 步骤 → 自动展开；否则看用户手动开关 */
function stepsOpen(i: number, m: ChatTurn): boolean {
  if (isStreaming(i) && hasRunning(m)) return true;
  return stepsExpanded.value[i] === true;
}

function toggleSteps(i: number) {
  stepsExpanded.value = { ...stepsExpanded.value, [i]: !(stepsExpanded.value[i] === true) };
}

/** 摘要行进行中文案：正在{类别} {参数}… */
function runningLabel(m: ChatTurn): string {
  const r = [...(m.steps ?? [])].reverse().find((s) => s.status === "running");
  if (!r) return "执行";
  const d = describeStep(r.name, r.args);
  return d.kind + (d.detail ? ` ${d.detail}` : "");
}

/** 步骤总耗时（首末时间戳差，旧数据无时间戳则不显示） */
function stepsElapsed(m: ChatTurn): string {
  const ts = (m.steps ?? []).map((s) => s.t).filter((t) => t > 0);
  if (ts.length < 2) return "";
  const ms = Math.max(...ts) - Math.min(...ts);
  return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`;
}

// ===== 单步详情（点击步骤行展开：完整参数 + 执行结果 JSON） =====
const stepDetail = ref<Record<string, boolean>>({});

function stepDetailOpen(i: number, si: number): boolean {
  return stepDetail.value[`${i}:${si}`] === true;
}

function toggleStepDetail(i: number, si: number) {
  const k = `${i}:${si}`;
  stepDetail.value = { ...stepDetail.value, [k]: !(stepDetail.value[k] === true) };
}

/** JSON 美化（解析失败原样返回） */
function fmtJson(s: string): string {
  try {
    return JSON.stringify(JSON.parse(s) as unknown, null, 2);
  } catch {
    return s;
  }
}

function prettyArgs(s: ToolStep): string {
  if (!s.args || s.args === "{}") return "";
  return fmtJson(s.args);
}

/** 结果 JSON 的 ok=false 视为失败（红叉 + 错误样式） */
function stepFailed(s: ToolStep): boolean {
  if (!s.result) return false;
  try {
    const v = JSON.parse(s.result) as { ok?: unknown };
    return v.ok === false;
  } catch {
    return false;
  }
}

function hasStepDetail(s: ToolStep): boolean {
  return !!(prettyArgs(s) || s.result);
}

// ===== 宽度拖拽（左缘把手，localStorage 持久化） =====
const WIDTH_KEY = "bm-ai-sidebar-width";
const MIN_W = 320;
const MAX_W = 720;

function loadWidth(): number {
  const raw = Number(localStorage.getItem(WIDTH_KEY));
  const w = Number.isFinite(raw) && raw > 0 ? raw : 400;
  return Math.max(MIN_W, Math.min(w, MAX_W));
}
const panelWidth = ref(loadWidth());
const dragging = ref(false);

function onResizeStart(e: MouseEvent) {
  e.preventDefault();
  dragging.value = true;
  const startX = e.clientX;
  const startW = panelWidth.value;
  const onMove = (ev: MouseEvent) => {
    panelWidth.value = Math.max(MIN_W, Math.min(startW + (startX - ev.clientX), MAX_W));
  };
  const onUp = () => {
    dragging.value = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    try {
      localStorage.setItem(WIDTH_KEY, String(panelWidth.value));
    } catch {
      /* 忽略 */
    }
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
  };
  document.body.style.cursor = "col-resize";
  document.body.style.userSelect = "none";
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
}

function resetWidth() {
  panelWidth.value = 400;
  try {
    localStorage.setItem(WIDTH_KEY, "400");
  } catch {
    /* 忽略 */
  }
}

watch(
  () => chat.messages.map((m) => `${m.content.length}:${m.steps?.length ?? 0}:${m.steps?.filter((s) => s.status === "running").length ?? 0}`).join(",") + "|" + chat.messages.length,
  () => nextTick(scrollBottom),
);
watch(
  () => chat.open,
  (v) => {
    if (v) nextTick(scrollBottom);
  },
);
// 新对话清空消息后重置折叠状态（索引不复用）
watch(
  () => chat.messages.length,
  (n) => {
    if (n === 0) stepsExpanded.value = {};
  },
);
</script>

<template>
  <Teleport to="body">
    <Transition name="ai-side">
      <aside v-if="chat.open" class="ai-sidebar" :class="{ dragging }" :style="{ width: panelWidth + 'px' }">
        <!-- 左缘宽度拖拽把手（双击复位 400） -->
        <div class="ai-resizer" title="拖动调节宽度 · 双击复位" @mousedown="onResizeStart" @dblclick="resetWidth" />
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
            <span class="ai-provider">{{ providerName }}</span>
          </div>
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
                  <!-- 工具调用步骤（ZCode 式折叠组：摘要行 + 展开明细） -->
                  <div v-if="m.steps?.length" class="steps">
                    <button class="steps-head" @click="toggleSteps(i)">
                      <LoadingOutlined v-if="hasRunning(m)" class="steps-spin" spin />
                      <svg v-else class="steps-ok" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                      <span class="steps-title">
                        {{ hasRunning(m) ? `正在${runningLabel(m)}…` : `执行了 ${m.steps.length} 个操作` }}
                      </span>
                      <span v-if="!hasRunning(m) && stepsElapsed(m)" class="steps-time">{{ stepsElapsed(m) }}</span>
                      <RightOutlined class="steps-chev" :class="{ open: stepsOpen(i, m) }" />
                    </button>
                    <div v-show="stepsOpen(i, m)" class="steps-body">
                      <template v-for="(s, si) in m.steps" :key="si">
                        <div
                          class="step-row"
                          :class="{ clickable: hasStepDetail(s), failed: stepFailed(s) }"
                          :title="hasStepDetail(s) ? '点击查看参数与结果' : undefined"
                          @click="hasStepDetail(s) && toggleStepDetail(i, si)"
                        >
                          <span class="step-status">
                            <LoadingOutlined v-if="s.status === 'running'" spin />
                            <svg v-else-if="stepFailed(s)" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">
                              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                            </svg>
                            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">
                              <polyline points="20 6 9 17 4 12" />
                            </svg>
                          </span>
                          <span class="step-icon">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" v-html="describeStep(s.name, s.args).icon" />
                          </span>
                          <span class="step-kind">{{ describeStep(s.name, s.args).kind }}</span>
                          <span v-if="describeStep(s.name, s.args).detail" class="step-detail">{{ describeStep(s.name, s.args).detail }}</span>
                          <RightOutlined v-if="hasStepDetail(s)" class="step-chev" :class="{ open: stepDetailOpen(i, si) }" />
                        </div>
                        <div v-if="stepDetailOpen(i, si)" class="step-box">
                          <div v-if="prettyArgs(s)" class="step-json">
                            <div class="step-json-label">参数</div>
                            <pre>{{ prettyArgs(s) }}</pre>
                          </div>
                          <div v-if="s.result" class="step-json" :class="{ err: stepFailed(s) }">
                            <div class="step-json-label">{{ stepFailed(s) ? "错误" : "结果" }}</div>
                            <pre>{{ fmtJson(s.result) }}</pre>
                          </div>
                        </div>
                      </template>
                    </div>
                  </div>

                  <MdStreamPreview :source="mdText(m, i)" :streaming="isStreaming(i)" :dark="themeStore.isDark" />

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

        <!-- 输入区：一体化 composer（chips 内嵌输入框底部） -->
        <footer class="ai-input">
          <div class="ai-composer">
            <textarea
              v-model="input"
              rows="1"
              placeholder="问点什么…（Enter 发送 · Alt+Enter 换行）"
              @keydown="onKey"
              @input="($event.target as HTMLTextAreaElement).style.height = 'auto'; ($event.target as HTMLTextAreaElement).style.height = Math.min(($event.target as HTMLTextAreaElement).scrollHeight, 120) + 'px'"
            />
            <div class="ai-bar">
              <a-dropdown trigger="['click']">
                <button class="bar-chip" :class="{ none: !currentModel }" title="点击切换模型">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="4" y="4" width="16" height="16" rx="2" /><rect x="9" y="9" width="6" height="6" /><line x1="9" y1="1" x2="9" y2="4" /><line x1="15" y1="1" x2="15" y2="4" /><line x1="9" y1="20" x2="9" y2="23" /><line x1="15" y1="20" x2="15" y2="23" /><line x1="20" y1="9" x2="23" y2="9" /><line x1="20" y1="14" x2="23" y2="14" /><line x1="1" y1="9" x2="4" y2="9" /><line x1="1" y1="14" x2="4" y2="14" />
                  </svg>
                  <span class="bar-chip-text">{{ currentModel || "选择模型" }}</span>
                  <DownOutlined class="bar-caret" />
                </button>
                <template #overlay>
                  <a-menu :selected-keys="currentModel ? [currentModel] : []" @click="onModelMenuClick">
                    <a-menu-item v-for="m in modelMenuItems" :key="m.key">{{ m.label }}</a-menu-item>
                  </a-menu>
                </template>
              </a-dropdown>
              <a-dropdown trigger="['click']">
                <button class="bar-chip" title="思考等级：关闭最快 / 适中默认 / 深入更严谨">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M12 2a7 7 0 0 0-7 7c0 2.38 1.19 4.47 3 5.74V17a2 2 0 0 0 2 2h4a2 2 0 0 0 2-2v-2.26c1.81-1.27 3-3.36 3-5.74a7 7 0 0 0-7-7z" /><line x1="10" y1="22" x2="14" y2="22" />
                  </svg>
                  <span class="bar-chip-text">思考·{{ thinkingLabel }}</span>
                  <DownOutlined class="bar-caret" />
                </button>
                <template #overlay>
                  <a-menu :selected-keys="[chat.thinking]" @click="onThinkingClick">
                    <a-menu-item key="off">关闭（最快）</a-menu-item>
                    <a-menu-item key="medium">适中（默认）</a-menu-item>
                    <a-menu-item key="high">深入（更严谨）</a-menu-item>
                  </a-menu>
                </template>
              </a-dropdown>
              <span class="bar-spacer" />
              <button v-if="chat.running" class="ai-send stop" title="停止" @click="chat.stop()">
                <StopOutlined />
              </button>
              <button v-else class="ai-send" :disabled="!input.trim()" title="发送" @click="send">
                <LoadingOutlined v-if="chat.running" /><SendOutlined v-else />
              </button>
            </div>
          </div>
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
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-panel, 14px);
  box-shadow: var(--shadow-panel-hover);
  overflow: hidden;
}

/* 左缘拖拽把手 */
.ai-resizer {
  position: absolute;
  left: -3px;
  top: 0;
  bottom: 0;
  width: 7px;
  cursor: col-resize;
  z-index: 5;
}
.ai-resizer::after {
  content: "";
  position: absolute;
  left: 3px;
  top: 0;
  bottom: 0;
  width: 1.5px;
  background: transparent;
  transition: background var(--transition-fast);
}
.ai-resizer:hover::after,
.ai-sidebar.dragging .ai-resizer::after {
  background: var(--color-primary);
}
.ai-sidebar.dragging {
  transition: none;
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

/* ===== 工具步骤折叠组（ZCode ToolCallBlock 模式） ===== */
.steps {
  margin-bottom: 8px;
  border: 1px solid var(--border-light);
  border-radius: 8px;
  background: color-mix(in srgb, var(--text-primary) 2%, transparent);
  overflow: hidden;
}
.steps-head {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  padding: 6px 10px;
  border: none;
  background: none;
  cursor: pointer;
  font: inherit;
  font-size: 11.5px;
  color: var(--text-secondary);
  text-align: left;
}
.steps-head:hover {
  background: color-mix(in srgb, var(--text-primary) 3%, transparent);
}
.steps-spin {
  font-size: 10px;
  color: var(--color-primary);
}
.steps-ok {
  width: 11px;
  height: 11px;
  flex-shrink: 0;
  color: #16a34a;
}
.steps-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.steps-time {
  flex-shrink: 0;
  font-size: 10px;
  color: var(--text-tertiary);
  font-variant-numeric: tabular-nums;
}
.steps-chev {
  margin-left: auto;
  flex-shrink: 0;
  font-size: 9px;
  color: var(--text-tertiary);
  transition: transform 0.15s ease;
}
.steps-chev.open {
  transform: rotate(90deg);
}
.steps-body {
  border-top: 1px solid var(--border-light);
  padding: 4px 0;
}
.step-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 10px;
  font-size: 11.5px;
  line-height: 1.4;
  color: var(--text-secondary);
  min-width: 0;
}
.step-status {
  display: flex;
  justify-content: center;
  flex-shrink: 0;
  width: 12px;
  color: #16a34a;
  font-size: 10px;
}
.step-status .anticon {
  font-size: 10px;
  color: var(--color-primary);
}
.step-status svg {
  width: 10px;
  height: 10px;
}
.step-icon {
  display: flex;
  flex-shrink: 0;
  color: var(--text-tertiary);
}
.step-icon svg {
  width: 12px;
  height: 12px;
}
.step-kind {
  flex-shrink: 0;
  font-weight: 500;
  color: var(--text-primary);
}
.step-detail {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 10.5px;
}
.step-row.clickable {
  cursor: pointer;
}
.step-row.clickable:hover {
  background: color-mix(in srgb, var(--text-primary) 3%, transparent);
}
.step-row.failed .step-kind {
  color: #dc2626;
}
.step-row.failed .step-status {
  color: #dc2626;
}
.step-chev {
  margin-left: auto;
  flex-shrink: 0;
  font-size: 8px;
  color: var(--text-tertiary);
  transition: transform 0.15s ease;
}
.step-chev.open {
  transform: rotate(90deg);
}
/* 单步详情面板：完整参数 + 结果 JSON */
.step-box {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 2px 10px 6px 30px;
}
.step-json-label {
  font-size: 10px;
  color: var(--text-tertiary);
  margin-bottom: 2px;
}
.step-json pre {
  margin: 0;
  padding: 6px 8px;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 10.5px;
  line-height: 1.5;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--text-primary) 3%, transparent);
  border: 1px solid var(--border-light);
  border-radius: 6px;
  max-height: 180px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
}
.step-json.err pre {
  color: #dc2626;
  border-color: color-mix(in srgb, #dc2626 30%, transparent);
}

/* Markdown：MdStreamPreview 自带紧凑样式，侧栏微调字号 */
.msg-body :deep(.md-lite) {
  font-size: 12.5px;
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

/* ===== 输入区：一体化 composer（chips 内嵌框内） ===== */
.ai-input {
  padding: 10px 12px;
  border-top: 1px solid var(--border-light);
  flex-shrink: 0;
}
.ai-composer {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 7px 8px 6px;
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: 12px;
  transition: border-color var(--transition-fast);
}
.ai-composer:focus-within {
  border-color: color-mix(in srgb, var(--color-primary) 45%, transparent);
}
.ai-composer textarea {
  width: 100%;
  min-height: 30px;
  max-height: 120px;
  padding: 2px 5px;
  font-size: 12.5px;
  font-family: inherit;
  line-height: 1.5;
  color: var(--text-primary);
  background: transparent;
  border: none;
  outline: none;
  resize: none;
}
.ai-composer textarea::placeholder {
  color: var(--text-tertiary);
}

/* composer 内工具行：模型 / 思考等级 ghost chips + 发送 */
.ai-bar {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
}
.bar-spacer {
  flex: 1;
}
.bar-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  max-width: 55%;
  padding: 4px 8px;
  font-size: 11px;
  font-family: inherit;
  color: var(--text-secondary);
  background: transparent;
  border: none;
  border-radius: 999px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.bar-chip:hover {
  color: var(--color-primary);
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
}
.bar-chip.none .bar-chip-text {
  color: #d97706;
}
.bar-chip svg {
  width: 11px;
  height: 11px;
  flex-shrink: 0;
}
.bar-chip-text {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bar-caret {
  font-size: 7px;
  flex-shrink: 0;
  opacity: 0.6;
}
.ai-send {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  font-size: 12px;
  color: var(--hero-cta-text, #fff);
  background: linear-gradient(135deg, var(--color-primary), color-mix(in srgb, var(--color-primary) 82%, #000));
  border: none;
  border-radius: 50%;
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
