<script setup lang="ts">
/**
 * AI 结果弹框：流式预览 + 应用操作
 *
 * - 流式期间：源码实时滚动 + 停止按钮 + 思考折叠块（若有）
 * - 文本动作完成：替换 / 追加 / 另存为新备忘录（emit("apply")，数据操作在父级）
 * - JSON 动作完成：标签 chips / 分类 chip + 采纳
 */
import { ref, computed, watch, nextTick } from "vue";
import { message } from "ant-design-vue";
import { ThunderboltOutlined, StopOutlined, LoadingOutlined } from "@ant-design/icons-vue";
import { streamAi, isCancelledError, type StreamHandle, type AiStreamChunk } from "@/api/ai";
import type { Memo } from "@/api/types";
import { findAction, parseAiTags, parseAiCategory, type AiActionDef } from "../aiActions";

const props = defineProps<{
  open: boolean;
  memo: Memo | null;
  action: AiActionDef | null;
  /** true = 作用于编辑弹框中未保存的内容：不显示「另存为新备忘录」 */
  editorMode?: boolean;
}>();
const emit = defineEmits<{
  (e: "update:open", v: boolean): void;
  (e: "apply", payload: { mode: string; result: string; action: AiActionDef }): void;
}>();

const running = ref(false);
const out = ref("");
const thinking = ref("");
const err = ref("");
const streamBox = ref<HTMLElement | null>(null);
let handle: StreamHandle | null = null;

const isTextMode = computed(() => props.action?.mode === "text");
const parsedTags = computed(() => (props.action?.mode === "tags" ? parseAiTags(out.value) : []));
const parsedCategory = computed(() =>
  props.action?.mode === "category" ? parseAiCategory(out.value) : "",
);
const parseFailed = computed(
  () =>
    props.action !== null &&
    !running.value &&
    !err.value &&
    !isTextMode.value &&
    !parsedTags.value.length &&
    !parsedCategory.value,
);

watch(
  () => props.open,
  (v) => {
    if (v && props.memo && props.action) start();
    if (!v) stopLocal();
  },
);

function start() {
  stopLocal();
  out.value = "";
  thinking.value = "";
  err.value = "";
  if (!props.memo || !props.action) return;
  running.value = true;
  handle = streamAi(
    {
      action: props.action.action,
      content: props.memo.content,
      title: props.memo.title,
      lang: props.action.action === "translate" ? props.action.label.replace("翻译为", "") : "",
    },
    (chunk: AiStreamChunk) => {
      if (chunk.type === "token") {
        out.value += chunk.content;
        nextTick(scrollBottom);
      } else if (chunk.type === "thinking") {
        thinking.value += chunk.content;
      } else if (chunk.type === "error") {
        err.value = chunk.message;
      }
    },
  );
  handle.done
    .then(() => {
      running.value = false;
    })
    .catch((e) => {
      running.value = false;
      if (!isCancelledError(e)) {
        err.value = e instanceof Error ? e.message : String(e);
      }
    });
}

function stopLocal() {
  handle?.cancel();
  handle = null;
  running.value = false;
}

function close() {
  stopLocal();
  emit("update:open", false);
}

function apply(mode: string) {
  if (!props.action) return;
  emit("apply", { mode, result: out.value, action: props.action });
  close();
}

function scrollBottom() {
  const el = streamBox.value;
  if (el) el.scrollTop = el.scrollHeight;
}
</script>

<template>
  <a-modal
    :open="open"
    :title="action ? `${action.label}` : 'AI'"
    :width="720"
    centered
    :footer="null"
    :mask-closable="!running"
    :closable="!running"
    wrap-class-name="ai-result-modal"
    @update:open="(v: boolean) => !v && close()"
  >
    <div class="ai-body">
      <div v-if="memo" class="ai-target" :title="memo.title || memo.content.slice(0, 60)">
        {{ editorMode ? "目标：编辑中（未保存）" : `目标：${memo.title || memo.content.slice(0, 40) + "…"}` }}
      </div>

      <!-- 思考通道（有才显示） -->
      <details v-if="thinking" class="ai-thinking">
        <summary>思考过程（{{ thinking.length }} 字）</summary>
        <pre>{{ thinking }}</pre>
      </details>

      <!-- 流式输出区 -->
      <div ref="streamBox" class="ai-stream">
        <pre v-if="out">{{ out }}<span v-if="running" class="caret" /></pre>
        <div v-else-if="running" class="ai-waiting">
          <LoadingOutlined /> 正在{{ action?.label }}…
        </div>
        <div v-else-if="!err" class="ai-waiting">（无输出）</div>
      </div>

      <div v-if="err" class="ai-error">{{ err }}</div>
      <div v-if="parseFailed" class="ai-error">输出无法解析为结构化结果，请重试。</div>

      <!-- 操作行 -->
      <div class="ai-actions">
        <template v-if="running">
          <span class="ai-state"><LoadingOutlined /> 生成中…</span>
          <span class="spacer" />
          <button class="btn-cancel" @click="stopLocal"><StopOutlined /> 停止</button>
        </template>
        <template v-else-if="isTextMode && out && !err">
          <span class="ai-state">{{ out.length }} 字</span>
          <span class="spacer" />
          <button class="btn-cancel" @click="close">丢弃</button>
          <button v-if="!editorMode" class="btn-cancel strong" @click="apply('saveNew')">另存为新备忘录</button>
          <button class="btn-cancel strong" @click="apply('append')">追加</button>
          <button class="btn-save" @click="apply('replace')">替换</button>
        </template>
        <template v-else-if="action?.mode === 'tags' && parsedTags.length">
          <div class="tag-adopt">
            <span v-for="t in parsedTags" :key="t" class="tag-chip-preview">#{{ t }}</span>
          </div>
          <span class="spacer" />
          <button class="btn-cancel" @click="close">丢弃</button>
          <button class="btn-save" @click="apply('adoptTags')">全部采纳</button>
        </template>
        <template v-else-if="action?.mode === 'category' && parsedCategory">
          <div class="tag-adopt">
            <span class="tag-chip-preview cat">分类：{{ parsedCategory }}</span>
          </div>
          <span class="spacer" />
          <button class="btn-cancel" @click="close">丢弃</button>
          <button class="btn-save" @click="apply('adoptCategory')">采纳</button>
        </template>
        <template v-else-if="!running && !err">
          <span class="spacer" />
          <button class="btn-cancel" @click="close">关闭</button>
          <button class="btn-cancel strong" @click="start"><ThunderboltOutlined /> 重试</button>
        </template>
      </div>
    </div>
  </a-modal>
</template>

<style scoped>
.ai-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.ai-target {
  font-size: 11.5px;
  color: var(--text-tertiary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ai-thinking {
  font-size: 11.5px;
  color: var(--text-tertiary);
  border: 1px dashed var(--border-light);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
}
.ai-thinking summary {
  cursor: pointer;
  user-select: none;
}
.ai-thinking pre {
  margin: 6px 0 0;
  max-height: 140px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: inherit;
  font-size: 11px;
  line-height: 1.6;
}
.ai-stream {
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  padding: 12px 14px;
  max-height: 46vh;
  min-height: 120px;
  overflow: auto;
}
.ai-stream pre {
  margin: 0;
  font-family: inherit;
  font-size: 12.5px;
  line-height: 1.7;
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}
.caret {
  display: inline-block;
  width: 7px;
  height: 14px;
  margin-left: 2px;
  vertical-align: -2px;
  background: var(--color-primary);
  animation: blink 1s step-end infinite;
}
@keyframes blink {
  50% {
    opacity: 0;
  }
}
.ai-waiting {
  display: grid;
  place-items: center;
  min-height: 96px;
  font-size: 12.5px;
  color: var(--text-tertiary);
  gap: 6px;
}
.ai-error {
  font-size: 12px;
  color: #ef4444;
  background: color-mix(in srgb, #ef4444 8%, transparent);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
}
.ai-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ai-state {
  font-size: 11px;
  color: var(--text-tertiary);
}
.spacer {
  flex: 1;
}
.tag-adopt {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  flex: 1;
}
.tag-chip-preview {
  font-size: 11.5px;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  border-radius: 999px;
  padding: 2px 10px;
}
.tag-chip-preview.cat {
  color: var(--text-primary);
}
.btn-cancel {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 30px;
  padding: 0 14px;
  font-size: 12.5px;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid var(--border-light);
  border-radius: 999px;
  cursor: pointer;
  transition: all var(--transition-fast);
  flex-shrink: 0;
}
.btn-cancel:hover {
  color: var(--text-primary);
  border-color: var(--text-tertiary);
}
.btn-cancel.strong {
  color: var(--text-primary);
  border-color: var(--text-tertiary);
}
.btn-save {
  height: 30px;
  padding: 0 18px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--hero-cta-text, #fff);
  background: linear-gradient(135deg, var(--color-primary), color-mix(in srgb, var(--color-primary) 82%, #000));
  border: none;
  border-radius: 999px;
  cursor: pointer;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--color-primary) 26%, transparent);
  transition: filter var(--transition-fast);
  flex-shrink: 0;
}
.btn-save:hover {
  filter: brightness(1.08);
}
</style>

<style>
/* 弹框容器主题化（teleport 到 body） */
.ai-result-modal .ant-modal-content {
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: 16px;
}
.ai-result-modal .ant-modal-header {
  background: transparent;
  border: none;
}
.ai-result-modal .ant-modal-title {
  font-family: "Noto Serif SC", "Source Han Serif SC", "STZhongsong", serif;
  font-size: 15px;
  font-weight: 600;
  letter-spacing: 0.03em;
  color: var(--text-primary);
}
.ai-result-modal .ant-modal-body {
  padding: 12px 20px 16px;
}
.ai-result-modal .ant-modal-close {
  color: var(--text-tertiary);
}
.ai-result-modal .ant-modal-close:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
}
</style>
