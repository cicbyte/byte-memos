<script setup lang="ts">
/**
 * Navbar 自绘标题栏（对齐 reference Navbar）
 *
 * 左：收缩按钮（紧贴 Sidebar 右侧） + 面包屑
 * 中：全局搜索框（输入时下拉浮层实时展示命中，点击跳转）
 * 右：明暗切换 + 设置 + 窗口控制
 */
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  MinusOutlined,
  BorderOutlined,
  CloseOutlined,
  SearchOutlined,
  SettingOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
} from "@ant-design/icons-vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useThemeStore } from "@/stores/theme";
import { useLayoutStore } from "@/stores/layout";
import { useAiChatStore } from "@/stores/aiChat";
import { api } from "@/api";
import type { UnifiedSearchHit, ItemType } from "@/api/types";

const route = useRoute();
const router = useRouter();
const themeStore = useThemeStore();
const layout = useLayoutStore();
const aiChat = useAiChatStore();
const q = ref("");
const appWindow = getCurrentWindow();

// ===== 搜索下拉浮层（跨类型合并）=====
const searchHits = ref<UnifiedSearchHit[]>([]);
const searching = ref(false);
const showDropdown = ref(false);
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

async function runSearch() {
  const query = q.value.trim();
  if (!query) {
    searchHits.value = [];
    showDropdown.value = false;
    return;
  }
  searching.value = true;
  try {
    searchHits.value = await api.searchAll(query, 12);
    showDropdown.value = true;
  } finally {
    searching.value = false;
  }
}

watch(q, () => {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(runSearch, 200);
});

function onFocusSearch() {
  if (q.value.trim() && searchHits.value.length) showDropdown.value = true;
}
function closeDropdown() {
  showDropdown.value = false;
}
function onPick(hit: UnifiedSearchHit) {
  // 备忘录：跳转到 /memos 打开对应编辑（?id=）
  if (hit.item_type === "memo") {
    router.push({ path: "/memos", query: { id: hit.short_id } });
  }
  closeDropdown();
  q.value = "";
}
function stripMarks(s: string): string {
  return s.replace(/<mark>/g, "").replace(/<\/mark>/g, "");
}

// 类型展示信息：标签文字 + CSS class（配色由 style 控制）
const TYPE_META: Record<ItemType, { label: string; cls: string }> = {
  memo: { label: "备忘录", cls: "type-memo" },
};

// 按 item_type 分组（保持 rank 升序，每组最多 4 条）
const groupedHits = computed(() => {
  const groups: Record<ItemType, UnifiedSearchHit[]> = {
    memo: [],
  };
  for (const h of searchHits.value) {
    groups[h.item_type].push(h);
  }
  return (["memo"] as ItemType[])
    .map((t) => ({ type: t, items: groups[t].slice(0, 4) }))
    .filter((g) => g.items.length > 0);
});

/** 面包屑：分组 / 页名 两级（来自 route.meta.group + title） */
const breadcrumbItems = computed(() => {
  const group = (route.meta.group as string) || "";
  const title = (route.meta.title as string) || "byte-memos";
  const items: { label: string }[] = [];
  if (group) items.push({ label: group });
  items.push({ label: title });
  return items;
});
</script>

<template>
  <div class="navbar" data-tauri-drag-region>
    <div class="navbar-left" data-tauri-drag-region>
      <!-- 收缩按钮：Navbar 最左侧，紧贴 Sidebar 右侧 -->
      <button
        class="sidebar-toggle"
        data-tauri-drag-region="false"
        :title="layout.sidebarCollapsed ? '展开菜单' : '收起菜单'"
        @click="layout.toggleSidebar()"
      >
        <MenuUnfoldOutlined v-if="layout.sidebarCollapsed" />
        <MenuFoldOutlined v-else />
      </button>

      <a-breadcrumb class="breadcrumb" separator="/" data-tauri-drag-region="false">
        <a-breadcrumb-item
          v-for="(item, idx) in breadcrumbItems"
          :key="idx"
          :class="{ 'crumb-current': idx === breadcrumbItems.length - 1 }"
        >
          {{ item.label }}
        </a-breadcrumb-item>
      </a-breadcrumb>
    </div>

    <div class="navbar-center" data-tauri-drag-region>
      <div class="search-wrapper" data-tauri-drag-region="false">
        <a-input
          v-model:value="q"
          placeholder="搜索备忘录…"
          class="search-box"
          allow-clear
          @focus="onFocusSearch"
          @keydown.esc="closeDropdown"
        >
          <template #prefix>
            <SearchOutlined style="color: var(--text-tertiary)" />
          </template>
        </a-input>

        <!-- 点击外部关闭浮层（透明遮罩） -->
        <div v-if="showDropdown" class="dropdown-mask" @mousedown="closeDropdown"></div>
        <!-- 下拉浮层：按类型分组的跨类型命中 -->
        <div v-if="showDropdown" class="search-dropdown">
          <div v-if="searching" class="dropdown-hint">搜索中…</div>
          <div v-else-if="!searchHits.length" class="dropdown-hint">无匹配</div>
          <template v-else>
            <div v-for="group in groupedHits" :key="group.type" class="dropdown-group">
              <div class="group-header">
                <span :class="['type-badge', TYPE_META[group.type].cls]">
                  {{ TYPE_META[group.type].label }}
                </span>
                <span class="group-count">{{ group.items.length }}</span>
              </div>
              <div
                v-for="hit in group.items"
                :key="hit.short_id"
                class="dropdown-item"
                @mousedown.prevent="onPick(hit)"
              >
                <div class="item-main">
                  <span class="item-id">{{ hit.short_id }}</span>
                  <span class="item-title">{{ hit.title }}</span>
                  <span v-if="hit.category" class="item-category">{{ hit.category }}</span>
                </div>
                <div v-if="stripMarks(hit.snippet)" class="item-snippet" v-html="hit.snippet"></div>
              </div>
            </div>
          </template>
        </div>
      </div>
    </div>

    <div class="navbar-right" data-tauri-drag-region>
      <button
        class="icon-btn"
        :class="{ 'ai-on': aiChat.open }"
        title="AI 助手"
        @click="aiChat.toggle()"
      >
        <svg class="theme-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z" />
          <path d="M20 3v4" /><path d="M22 5h-4" /><path d="M4 17v2" /><path d="M5 18H3" />
        </svg>
      </button>
      <button
        class="icon-btn"
        :title="themeStore.isDark ? '切换到浅色' : '切换到深色'"
        @click="themeStore.setMode(themeStore.isDark ? 'light' : 'dark')"
      >
        <!-- 暗色时显示太阳（点击切到亮色）；亮色时显示月亮（点击切到暗色）。Feather 线性图标，currentColor 跟随文字色 -->
        <svg v-if="themeStore.isDark" class="theme-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="5" />
          <line x1="12" y1="1" x2="12" y2="3" />
          <line x1="12" y1="21" x2="12" y2="23" />
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
          <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
          <line x1="1" y1="12" x2="3" y2="12" />
          <line x1="21" y1="12" x2="23" y2="12" />
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
          <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
        </svg>
        <svg v-else class="theme-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
      </button>
      <button
        class="icon-btn"
        title="设置"
        @click="router.push('/settings')"
      >
        <SettingOutlined />
      </button>
      <span class="divider" />
      <button class="win-btn" title="最小化" @click="appWindow.minimize()">
        <MinusOutlined />
      </button>
      <button class="win-btn" title="最大化" @click="appWindow.toggleMaximize()">
        <BorderOutlined />
      </button>
      <button class="win-btn win-close" title="关闭" @click="appWindow.close()">
        <CloseOutlined />
      </button>
    </div>
  </div>
</template>

<style scoped>
.navbar {
  height: var(--navbar-height);
  display: flex;
  align-items: center;
  padding: 0 var(--spacing-md);
  /* 悬浮面板体系：底色/边框由 AppLayout 的 shell-panel 提供 */
  flex-shrink: 0;
  user-select: none;
}
.navbar-left {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  min-width: 120px;
}
.sidebar-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  border-radius: var(--radius-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
  flex-shrink: 0;
  font-size: 15px;
}
.sidebar-toggle:hover {
  background: var(--bg-elevated);
  color: var(--color-primary);
}
.navbar-center {
  flex: 1;
  display: flex;
  justify-content: center;
  padding: 0 var(--spacing-md);
}
.search-box {
  max-width: 420px;
  width: 100%;
}
/* 搜索下拉浮层 */
.search-wrapper {
  position: relative;
  width: 100%;
  max-width: 420px;
}
.dropdown-mask {
  position: fixed;
  inset: 0;
  z-index: 1090;
}
.search-dropdown {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  max-height: 360px;
  overflow-y: auto;
  background: var(--bg-elevated);
  border: 1px solid var(--border-base);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  padding: 4px;
  z-index: 1100;
}
.dropdown-hint {
  padding: 10px 12px;
  font-size: 12px;
  color: var(--text-tertiary);
  text-align: center;
}
.dropdown-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 7px 10px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background var(--transition-fast);
}
.dropdown-item:hover {
  background: var(--color-primary-bg);
}
.dropdown-group {
  padding: 2px 0;
}
.dropdown-group + .dropdown-group {
  border-top: 1px solid var(--border-light);
  margin-top: 2px;
  padding-top: 4px;
}
.group-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px 2px;
}
.type-badge {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: var(--radius-xs);
  line-height: 1.5;
}
/* 类型配色：备忘录=主色（color-mix 派生，与主题系统一致） */
.type-memo {
  color: var(--color-primary);
  background: color-mix(in oklch, var(--color-primary) 14%, transparent);
}
.group-count {
  font-size: 11px;
  color: var(--text-tertiary);
}
.item-main {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.item-id {
  font-family: "JetBrains Mono", monospace;
  font-size: 11px;
  color: var(--color-primary);
  flex-shrink: 0;
}
.item-title {
  font-size: 13px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}
.item-category {
  font-size: 11px;
  color: var(--text-tertiary);
  flex-shrink: 0;
}
.item-snippet {
  font-size: 11px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding-left: 0;
}
.item-snippet :deep(mark) {
  color: var(--color-primary);
  background: transparent;
  font-weight: 600;
}
.navbar-right {
  display: flex;
  align-items: center;
  gap: 2px;
  min-width: 120px;
  justify-content: flex-end;
}
.breadcrumb {
  font-size: 13px;
  color: var(--text-secondary);
}
/* 上级分组：弱色 */
.breadcrumb :deep(.ant-breadcrumb-link) {
  color: var(--text-tertiary);
}
/* 分隔符：弱化 */
.breadcrumb :deep(.ant-breadcrumb-separator) {
  color: var(--text-disabled);
}
/* 当前页：主色加粗，清晰层级 */
.breadcrumb :deep(.crumb-current > .ant-breadcrumb-link) {
  color: var(--color-primary);
  font-weight: 600;
}
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 15px;
  transition: all var(--transition-fast);
}
.theme-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}
.icon-btn:hover {
  background: var(--bg-elevated);
  color: var(--text-primary);
}
/* AI 入口：打开态主色高亮 */
.icon-btn.ai-on {
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
}
.divider {
  width: 1px;
  height: 18px;
  background: var(--border-base);
  margin: 0 4px;
}
.win-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: var(--radius-xs);
  font-size: 13px;
  transition: all var(--transition-fast);
}
.win-btn:hover {
  background: var(--bg-elevated);
  color: var(--text-primary);
}
.win-close:hover {
  background: var(--color-danger);
  color: #fff;
}
</style>
