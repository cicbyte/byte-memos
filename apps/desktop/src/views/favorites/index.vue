<script setup lang="ts">
/**
 * 收藏页 /favorites
 *
 * 展示被收藏的书签，点击跳转到书签页对应项（?id=）。
 */
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { LinkOutlined, StarOutlined } from "@ant-design/icons-vue";
import { api } from "@/api";
import EmptyState from "@/components/common/EmptyState.vue";
import type { Bookmark } from "@/api/types";
import { useLayoutStore } from "@/stores/layout";

const router = useRouter();
const layout = useLayoutStore();

const items = ref<Bookmark[]>([]);
const loading = ref(false);

const filtered = computed(() => items.value);

function fmtDate(iso: string): string {
  return iso ? iso.slice(0, 10) : "";
}

async function load() {
  loading.value = true;
  try {
    const bms = await api.listBookmarks({ favorited_only: true, limit: 500 });
    // 按创建时间降序
    items.value = [...bms].sort((a, b) => b.created_at.localeCompare(a.created_at));
    layout.setStatusItems([
      { key: "view", text: "收藏" },
      { key: "count", text: `共 ${items.value.length} 项` },
    ]);
  } finally {
    loading.value = false;
  }
}

/** 跳转到书签页对应项 */
function goTo(it: Bookmark) {
  router.push({ path: "/bookmarks", query: { id: it.short_id } });
}

onMounted(load);
</script>

<template>
  <div class="fav-view">
    <!-- 顶部工具栏 -->
    <div class="toolbar">
      <span class="page-title">收藏</span>
      <span class="count-badge">{{ items.length }}</span>
    </div>

    <!-- 列表面板 -->
    <section class="panel">
      <header class="panel-head">
        <span class="head-title">已收藏书签</span>
        <span class="count-badge">{{ filtered.length }}</span>
      </header>

      <div class="panel-body" style="position:relative">
          <div
            v-for="it in filtered"
            :key="it.short_id"
            class="fav-item"
            @click="goTo(it)"
          >
            <LinkOutlined class="item-icon bookmark" />
            <div class="item-main">
              <div class="item-title">{{ it.title || it.url || "(无标题)" }}</div>
              <div class="item-sub">
                <span v-if="it.description" class="sub-desc">{{ it.description }}</span>
                <span v-if="it.category" class="sep">·</span>
                <span v-if="it.category" class="sub-meta">{{ it.category }}</span>
              </div>
            </div>
            <span class="item-date">{{ fmtDate(it.created_at) }}</span>
          </div>

          <EmptyState
            v-if="!filtered.length && !loading"
            :icon="StarOutlined"
            text="暂无收藏"
            hint="在书签页点击 ☆ 收藏常用书签"
            size="small"
          />
      </div>
    </section>
  </div>
</template>

<style scoped>
.fav-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: var(--spacing-md);
  gap: var(--spacing-md);
}

/* 工具栏 */
.toolbar {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
  flex-shrink: 0;
  height: 44px;
}
.page-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  min-width: 56px;
  white-space: nowrap;
}
.count-badge {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
  background: var(--bg-input);
  padding: 1px 7px;
  border-radius: 10px;
}

/* 主面板 */
.panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-lg);
  overflow: hidden;
  min-height: 0;
}
.panel-head {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  height: 44px;
  padding: 0 var(--spacing-md);
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-surface);
  flex-shrink: 0;
}
.head-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
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
.item-icon.bookmark { color: var(--color-primary); }

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
