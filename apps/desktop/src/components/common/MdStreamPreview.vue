<script setup lang="ts">
/**
 * 流式 Markdown 预览（markdown-it + highlight.js 本地渲染，120ms 节流）
 *
 * 移植自 byte-writer MdStreamPreview 并增强语法高亮。不用 md-editor-v3 的
 * MdPreview：主题 chrome（mac 栏/行号/22px 留白）依赖全局 CSS 注入且难以驯服，
 * 流式下每 token 全量重解析开销大。html:false 使 AI 输出的 HTML 标签
 * 字面显示不执行（防注入）。
 */
import { computed, onBeforeUnmount, ref, watch } from "vue";
import MarkdownIt from "markdown-it";
import hljs from "highlight.js/lib/common";

const props = defineProps<{
  source: string;
  streaming?: boolean;
  dark?: boolean;
}>();

const md = new MarkdownIt({ html: false, breaks: true, linkify: true });

// 链接交系统浏览器打开（避免 webview 内导航离开应用）
const defaultLinkOpen =
  md.renderer.rules.link_open ?? ((t, i, o, e, s) => s.renderToken(t, i, o));
md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
  tokens[idx].attrSet("target", "_blank");
  tokens[idx].attrSet("rel", "noopener");
  return defaultLinkOpen(tokens, idx, options, env, self);
};

// 代码块：highlight.js 上色 + data-lang 语言标签（CSS 画右上角小徽标）
md.renderer.rules.fence = (tokens, idx) => {
  const t = tokens[idx];
  const lang = (t.info || "").trim().split(/\s+/)[0];
  const tag = lang ? ` data-lang="${md.utils.escapeHtml(lang)}"` : "";
  if (lang && hljs.getLanguage(lang)) {
    try {
      const highlighted = hljs.highlight(t.content, { language: lang, ignoreIllegals: true }).value;
      return `<pre${tag}><code class="hljs">${highlighted}</code></pre>`;
    } catch {
      /* 高亮失败退回原文 */
    }
  }
  return `<pre${tag}><code class="hljs">${md.utils.escapeHtml(t.content)}</code></pre>`;
};

const shown = ref(props.source);
let timer: ReturnType<typeof setTimeout> | null = null;

watch(
  () => props.source,
  (v) => {
    if (!props.streaming) {
      shown.value = v;
      return;
    }
    if (timer) return;
    timer = setTimeout(() => {
      timer = null;
      shown.value = props.source;
    }, 120);
  },
);

watch(
  () => props.streaming,
  (s) => {
    if (!s) {
      if (timer) clearTimeout(timer);
      timer = null;
      shown.value = props.source;
    }
  },
);

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer);
});

const html = computed(() => md.render(shown.value || ""));
</script>

<template>
  <div class="md-lite" :class="{ dark }" v-html="html"></div>
</template>

<style scoped>
.md-lite {
  /* highlight token 配色（github light），dark 命名空间翻转 */
  --hl-keyword: #cf222e;
  --hl-string: #0a3069;
  --hl-number: #0550ae;
  --hl-comment: #6e7781;
  --hl-func: #8250df;
  --hl-attr: #953800;
  --hl-type: #116329;
  --hl-meta: #57606a;

  font-size: 13px;
  line-height: 1.7;
  color: var(--text-primary);
  word-break: break-word;
}
.md-lite.dark {
  --hl-keyword: #ff7b72;
  --hl-string: #a5d6ff;
  --hl-number: #79c0ff;
  --hl-comment: #8b949e;
  --hl-func: #d2a8ff;
  --hl-attr: #ffa657;
  --hl-type: #7ee787;
  --hl-meta: #8b949e;
}

.md-lite :deep(:first-child) {
  margin-top: 0;
}
.md-lite :deep(:last-child) {
  margin-bottom: 0;
}
.md-lite :deep(p) {
  margin-block: 8px;
}
.md-lite :deep(h1),
.md-lite :deep(h2),
.md-lite :deep(h3),
.md-lite :deep(h4) {
  margin: 14px 0 7px;
  font-weight: 650;
  line-height: 1.4;
}
.md-lite :deep(h1),
.md-lite :deep(h2) {
  padding-bottom: 4px;
  border-bottom: 1px solid var(--border-light);
}
.md-lite :deep(h1) {
  font-size: 16.5px;
}
.md-lite :deep(h2) {
  font-size: 15px;
}
.md-lite :deep(h3) {
  font-size: 14px;
}
.md-lite :deep(h4) {
  font-size: 13.5px;
}
.md-lite :deep(ul),
.md-lite :deep(ol) {
  margin: 8px 0;
  padding-left: 20px;
}
.md-lite :deep(li) {
  margin: 3px 0;
}
.md-lite :deep(li > p) {
  margin: 0;
}
.md-lite :deep(strong) {
  font-weight: 650;
}

/* 行内代码：主色 tint */
.md-lite :deep(code) {
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 0.92em;
  padding: 1px 5px;
  border-radius: 4px;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 8%, transparent);
}

/* 代码块：紧凑卡片 + 右上角语言标签 */
.md-lite :deep(pre) {
  position: relative;
  margin: 8px 0;
  padding: 10px 12px;
  border-radius: 8px;
  overflow-x: auto;
  background: var(--bg-input, rgba(127, 127, 127, 0.12));
  border: 1px solid var(--border-light);
}
.md-lite :deep(pre[data-lang])::before {
  content: attr(data-lang);
  position: absolute;
  top: 6px;
  right: 8px;
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 10px;
  font-weight: 500;
  line-height: 1;
  color: var(--text-tertiary);
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  padding: 3px 7px;
  border-radius: 4px;
  pointer-events: none;
  user-select: none;
}
.md-lite :deep(pre code) {
  padding: 0;
  background: none;
  color: var(--text-primary);
  font-size: 11.5px;
  line-height: 1.65;
}
.md-lite :deep(pre code.hljs) {
  padding: 0;
  background: transparent;
}

/* highlight token 配色（CSS 变量随明暗翻转） */
.md-lite :deep(.hljs-keyword),
.md-lite :deep(.hljs-selector-tag),
.md-lite :deep(.hljs-deletion),
.md-lite :deep(.hljs-tag) {
  color: var(--hl-keyword);
}
.md-lite :deep(.hljs-string),
.md-lite :deep(.hljs-regexp),
.md-lite :deep(.hljs-quote) {
  color: var(--hl-string);
}
.md-lite :deep(.hljs-addition) {
  color: var(--hl-type);
}
.md-lite :deep(.hljs-number),
.md-lite :deep(.hljs-literal),
.md-lite :deep(.hljs-symbol),
.md-lite :deep(.hljs-bullet) {
  color: var(--hl-number);
}
.md-lite :deep(.hljs-comment) {
  color: var(--hl-comment);
  font-style: italic;
}
.md-lite :deep(.hljs-title),
.md-lite :deep(.hljs-title.function_),
.md-lite :deep(.hljs-section),
.md-lite :deep(.hljs-name) {
  color: var(--hl-func);
}
.md-lite :deep(.hljs-attr),
.md-lite :deep(.hljs-attribute),
.md-lite :deep(.hljs-variable),
.md-lite :deep(.hljs-template-variable),
.md-lite :deep(.hljs-property),
.md-lite :deep(.hljs-selector-attr),
.md-lite :deep(.hljs-selector-class),
.md-lite :deep(.hljs-selector-id) {
  color: var(--hl-attr);
}
.md-lite :deep(.hljs-type),
.md-lite :deep(.hljs-title.class_),
.md-lite :deep(.hljs-built_in),
.md-lite :deep(.hljs-builtin-name) {
  color: var(--hl-type);
}
.md-lite :deep(.hljs-meta),
.md-lite :deep(.hljs-doctag),
.md-lite :deep(.hljs-params) {
  color: var(--hl-meta);
}
.md-lite :deep(.hljs-strong) {
  font-weight: 700;
}
.md-lite :deep(.hljs-emphasis) {
  font-style: italic;
}

.md-lite :deep(blockquote) {
  margin: 8px 0;
  padding: 2px 10px;
  border-left: 2px solid color-mix(in srgb, var(--color-primary) 55%, transparent);
  background: color-mix(in srgb, var(--color-primary) 5%, transparent);
  border-radius: 0 6px 6px 0;
  color: var(--text-secondary);
}
.md-lite :deep(a) {
  color: var(--color-primary, #1677ff);
  text-decoration: none;
}
.md-lite :deep(a:hover) {
  text-decoration: underline;
}
.md-lite :deep(table) {
  border-collapse: collapse;
  margin: 8px 0;
  font-size: 12px;
  width: 100%;
  display: block;
  overflow-x: auto;
}
.md-lite :deep(th),
.md-lite :deep(td) {
  border: 1px solid var(--border-light);
  padding: 5px 9px;
  text-align: left;
}
.md-lite :deep(th) {
  background: var(--bg-input, rgba(127, 127, 127, 0.1));
  font-weight: 600;
  white-space: nowrap;
}
.md-lite :deep(hr) {
  border: none;
  border-top: 1px solid var(--border-light);
  margin: 12px 0;
}
.md-lite :deep(img) {
  max-width: 100%;
  border-radius: 6px;
}
</style>
