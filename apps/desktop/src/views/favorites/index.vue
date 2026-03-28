<script setup lang="ts">
/**
 * 置顶页 /favorites
 *
 * 展示置顶的备忘录，点击跳转到备忘录页对应项（?id=）。
 */
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { FileTextOutlined, ReloadOutlined } from "@ant-design/icons-vue";
import { api } from "@/api";
import EmptyState from "@/components/common/EmptyState.vue";
import type { Memo } from "@/api/types";
import { useLayoutStore } from "@/stores/layout";

const router = useRouter();
const layout = useLayoutStore();

const items = ref<Memo[]>([]);
const loading = ref(false);

const filtered = computed(() => items.value);

/** Markdown → 纯文本摘要 */
function excerpt(m: Memo, max = 100): string {
  const text = m.content
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/[#>*_~`\-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return text.length > max ? `${text.slice(0, max)}…` : text;
}

function fmtDate(iso: string): string {
  return iso ? iso.slice(0, 10) : "";
}

async function load() {
  loading.value = true;
  try {
    const memos = await api.listMemos({ pinned_only: true, limit: 500 });
    // 按创建时间降序
    items.value = [...memos].sort((a, b) => b.created_at.localeCompare(a.created_at));
    layout.setStatusItems([
      { key: "view", text: "置顶" },
      { key: "count", text: `共 ${items.value.length} 篇` },
    ]);
  } finally {
    loading.value = false;
  }
}

/** 跳转到备忘录页对应项 */
function goTo(it: Memo) {
  router.push({ path: "/memos", query: { id: it.short_id } });
}

onMounted(load);
</script>

<template>
  <div class="fav-view">
    <!-- 单面板 + panel-head（对齐备忘录/附件库） -->
    <section class="panel">
      <header class="panel-head">
        <span class="head-title">置顶</span>
        <span class="head-count">{{ items.length }}</span>
        <div class="head-spacer" />
        <div class="head-actions">
          <button class="icon-action" title="刷新" @click="load"><ReloadOutlined /></button>
        </div>
      </header>

      <div class="panel-body" style="position:relative">
          <div
            v-for="it in filtered"
            :key="it.short_id"
            class="fav-item"
            @click="goTo(it)"
          >
            <FileTextOutlined class="item-icon memo" />
            <div class="item-main">
              <div class="item-title">{{ it.title || "(无标题)" }}</div>
              <div class="item-sub">
                <span v-if="excerpt(it)" class="sub-desc">{{ excerpt(it) }}</span>
                <span v-if="it.category" class="sep">·</span>
                <span v-if="it.category" class="sub-meta">{{ it.category }}</span>
              </div>
            </div>
            <span class="item-date">{{ fmtDate(it.created_at) }}</span>
          </div>

          <EmptyState
            v-if="!filtered.length && !loading"
            name="star"
            text="暂无置顶备忘录"
            desc="在备忘录卡片操作里选「置顶」，常用内容会出现在这里"
            size="small"
          />
      </div>
    </section>
  </div>
</template>

<style scoped>
/* ===== 页面容器（对齐备忘录/附件库） ===== */
.fav-view {
  height: 100%;
  padding: var(--spacing-lg);
  overflow: auto;
}
.panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  overflow: hidden;
}
.panel-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-light);
  flex-shrink: 0;
}
.head-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}
.head-count {
  font-size: 12px;
  color: var(--text-tertiary);
  background: var(--bg-input);
  padding: 2px 8px;
  border-radius: 10px;
}
.head-spacer {
  flex: 1;
}
.head-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}
.icon-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  font-size: 12px;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.icon-action:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
}
.panel-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-sm);
}

/* 列表项 */
.fav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--transition-fast);
}
.fav-item:hover {
  background: var(--bg-elevated);
}
.item-icon {
  font-size: 16px;
  flex-shrink: 0;
  width: 20px;
  text-align: center;
}
.item-icon.memo { color: var(--color-primary); }

.item-main {
  flex: 1;
  min-width: 0;
}
.item-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.item-sub {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 2px;
  font-size: 11px;
  color: var(--text-tertiary);
  overflow: hidden;
}
.sub-desc {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 320px;
}
.sep {
  color: var(--text-disabled);
}
.item-date {
  font-size: 11px;
  color: var(--text-disabled);
  font-family: "JetBrains Mono", monospace;
  flex-shrink: 0;
  min-width: 70px;
  text-align: right;
}
</style>
