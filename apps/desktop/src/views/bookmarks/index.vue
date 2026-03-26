<script setup lang="ts">
/**
 * 书签页 /bookmarks（byte-link 核心领域）
 *
 * 列表 / 卡片两种视图（工具栏切换，localStorage 持久化）
 * + 新增/编辑抽屉 + 本地实时搜索 + 打开/收藏/归档/删除。
 * 复用主题 CSS 变量；favicon 优先本地缓存（convertFileSrc），缺失时在线占位（F11）。
 */
import { ref, computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import {
  PlusOutlined,
  ReloadOutlined,
  StarFilled,
  StarOutlined,
  EditOutlined,
  DeleteOutlined,
  GlobalOutlined,
  InboxOutlined,
  SearchOutlined,
  AppstoreOutlined,
  UnorderedListOutlined,
} from "@ant-design/icons-vue";
import { message, Modal } from "ant-design-vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api } from "@/api";
import type { Bookmark, Category } from "@/api/types";
import { useLayoutStore } from "@/stores/layout";

const layout = useLayoutStore();
const route = useRoute();
const bookmarks = ref<Bookmark[]>([]);
const categories = ref<Category[]>([]);
const allTags = ref<string[]>([]);
const loading = ref(false);
const dataDir = ref("");

const searchQuery = ref("");
const filterCat = ref<string>("");
const filterArchived = ref<"active" | "archived" | "all">("active");

/** 视图模式：list（行）/ card（网格），localStorage 持久化 */
const viewMode = ref<"list" | "card">(
  (localStorage.getItem("bm-view-mode") as "list" | "card") || "list",
);
function setView(m: "list" | "card") {
  viewMode.value = m;
  localStorage.setItem("bm-view-mode", m);
}

// 新增/编辑抽屉
const drawerOpen = ref(false);
const editing = ref<Bookmark | null>(null);
const saving = ref(false);
const form = ref({
  url: "",
  title: "",
  description: "",
  category: undefined as string | undefined,
  tags: [] as string[],
});

/** 本地实时过滤（已加载列表内） */
const filtered = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  let list = bookmarks.value;
  if (filterCat.value) list = list.filter((b) => b.category === filterCat.value);
  if (!q) return list;
  return list.filter(
    (b) =>
      b.title.toLowerCase().includes(q) ||
      b.url.toLowerCase().includes(q) ||
      b.description.toLowerCase().includes(q) ||
      b.tags.some((t) => t.toLowerCase().includes(q)),
  );
});

async function load() {
  loading.value = true;
  try {
    const archived =
      filterArchived.value === "all" ? undefined : filterArchived.value === "archived";
    bookmarks.value = await api.listBookmarks({
      category: filterCat.value || undefined,
      archived,
      limit: 500,
    });
    try {
      categories.value = await api.listCategories();
    } catch {
      categories.value = [];
    }
    updateFooter();
    // 支持 ?id= 路由参数（收藏页 / 全局搜索跳转）→ 打开编辑
    const id = route.query.id as string | undefined;
    if (id) {
      try {
        const b = await api.getBookmark(id);
        openEdit(b);
      } catch {
        /* 忽略 */
      }
    }
  } catch (e: any) {
    message.error(typeof e === "string" ? e : "加载失败");
  } finally {
    loading.value = false;
  }
}

function updateFooter() {
  layout.setStatusItems([
    { key: "view", text: "书签" },
    { key: "count", text: `共 ${bookmarks.value.length} 个` },
  ]);
}

function openUrl(b: Bookmark) {
  api.incrementBookmarkVisit(b.short_id).catch(() => {});
  api.openUrl(b.url);
}

async function toggleFav(b: Bookmark) {
  try {
    patchInList(await api.toggleBookmarkFavorite(b.short_id));
  } catch {
    message.error("操作失败");
  }
}

async function toggleArchive(b: Bookmark) {
  try {
    const u = await api.setBookmarkArchived(b.short_id, !b.is_archived);
    patchInList(u);
    message.success(u.is_archived ? "已归档" : "已取消归档");
    if (filterArchived.value !== "all") await load();
  } catch {
    message.error("操作失败");
  }
}

function remove(b: Bookmark) {
  Modal.confirm({
    title: "移入回收站",
    content: `确认将「${b.title}」移入回收站？可在回收站恢复。`,
    okType: "danger",
    onOk: async () => {
      await api.deleteBookmark(b.short_id);
      message.success("已移入回收站");
      bookmarks.value = bookmarks.value.filter((x) => x.short_id !== b.short_id);
      updateFooter();
    },
  });
}

async function loadTags() {
  try {
    allTags.value = (await api.tagStats(100)).map((t) => t.tag);
  } catch {
    allTags.value = [];
  }
}

function openAdd() {
  editing.value = null;
  form.value = {
    url: "",
    title: "",
    description: "",
    category: filterCat.value || undefined,
    tags: [],
  };
  loadTags();
  drawerOpen.value = true;
}

function openEdit(b: Bookmark) {
  editing.value = b;
  form.value = {
    url: b.url,
    title: b.title,
    description: b.description,
    category: b.category ?? undefined,
    tags: [...b.tags],
  };
  loadTags();
  drawerOpen.value = true;
}

async function save() {
  if (!form.value.url.trim()) {
    message.warning("请输入网址");
    return;
  }
  saving.value = true;
  try {
    if (editing.value) {
      const u = await api.updateBookmark(editing.value.short_id, {
        title: form.value.title.trim() || form.value.url.trim(),
        url: form.value.url.trim(),
        description: form.value.description,
        category: form.value.category,
        tags: form.value.tags,
        archived: editing.value.is_archived,
      });
      patchInList(u);
      message.success("已保存");
    } else {
      const created = await api.addBookmark({
        url: form.value.url.trim(),
        title: form.value.title.trim() || undefined,
        description: form.value.description || undefined,
        category: form.value.category,
        tags: form.value.tags.length ? form.value.tags : undefined,
      });
      bookmarks.value = [created, ...bookmarks.value];
      message.success("已添加");
      // 抓取 favicon（best-effort，不阻塞；成功后更新本地 favicon_path 触发图标重渲染）
      void api.fetchFavicon(created.short_id).then((rel) => {
        if (rel) {
          bookmarks.value = bookmarks.value.map((b) =>
            b.short_id === created.short_id ? { ...b, favicon_path: rel } : b,
          );
        }
      });
    }
    drawerOpen.value = false;
    try {
      categories.value = await api.listCategories();
    } catch {
      /* 忽略 */
    }
    updateFooter();
  } catch (e: any) {
    message.error(typeof e === "string" ? e : "保存失败");
  } finally {
    saving.value = false;
  }
}

function patchInList(u: Bookmark) {
  const idx = bookmarks.value.findIndex((b) => b.short_id === u.short_id);
  if (idx >= 0) bookmarks.value = bookmarks.value.map((b, i) => (i === idx ? u : b));
}

function host(url: string): string {
  return url.replace(/^https?:\/\//, "").split("/")[0] || url;
}

/** favicon：优先本地缓存（convertFileSrc → webview 可访问），缺失则在线服务占位（onerror 降级隐藏） */
function faviconUrl(b: Bookmark): string {
  if (b.favicon_path && dataDir.value) {
    return convertFileSrc(`${dataDir.value}/${b.favicon_path}`);
  }
  return `https://www.google.com/s2/favicons?domain=${encodeURIComponent(host(b.url))}&sz=32`;
}

onMounted(async () => {
  try {
    dataDir.value = await api.dataDir();
  } catch {
    /* ignore */
  }
  load();
});
</script>

<template>
  <div class="bm-view">
    <section class="panel">
      <header class="panel-head">
        <span class="head-title">书签</span>
        <span class="head-count">{{ bookmarks.length }}</span>

        <div class="head-search">
          <SearchOutlined class="search-icon" />
          <input
            v-model="searchQuery"
            placeholder="搜索书签…"
          />
        </div>

        <a-select v-model:value="filterCat" size="small" style="width: 140px" placeholder="全部分类" allow-clear @change="load">
          <a-select-option value="">全部分类</a-select-option>
          <a-select-option v-for="c in categories" :key="c.id" :value="c.name">{{ c.name }}</a-select-option>
        </a-select>

        <a-select v-model:value="filterArchived" size="small" style="width: 100px" @change="load">
          <a-select-option value="active">活跃</a-select-option>
          <a-select-option value="archived">已归档</a-select-option>
          <a-select-option value="all">全部</a-select-option>
        </a-select>

        <div class="view-toggle">
          <button class="vt-btn" :class="{ on: viewMode === 'list' }" title="列表视图" @click="setView('list')"><UnorderedListOutlined /></button>
          <button class="vt-btn" :class="{ on: viewMode === 'card' }" title="卡片视图" @click="setView('card')"><AppstoreOutlined /></button>
        </div>

        <div class="head-actions">
          <button class="icon-action" title="刷新" @click="load"><ReloadOutlined /></button>
          <button class="icon-action primary" title="新增书签" @click="openAdd"><PlusOutlined /></button>
        </div>
      </header>

      <div class="panel-body">
        <div v-if="loading" class="bm-empty">加载中…</div>
        <div v-else-if="!filtered.length" class="bm-empty">
          {{ searchQuery || filterCat ? "无匹配书签" : "暂无书签 — 点 + 添加" }}
        </div>

        <!-- 列表视图 -->
        <div v-else-if="viewMode === 'list'" class="bm-list">
          <div
            v-for="b in filtered"
            :key="b.short_id"
            class="bm-item"
            :class="{ archived: b.is_archived }"
          >
            <img class="bm-fav" :src="faviconUrl(b)" @error="(e: any) => (e.target.style.visibility = 'hidden')" />
            <div class="bm-main" @click="openUrl(b)">
              <div class="bm-title-line">
                <span class="bm-title">{{ b.title }}</span>
                <StarFilled v-if="b.is_favorited" class="bm-fav-mark" />
                <InboxOutlined v-if="b.is_archived" class="bm-arch-mark" />
              </div>
              <div class="bm-url">{{ b.url }}</div>
              <div class="bm-tags">
                <a-tag v-if="b.category" size="small" color="geekblue">{{ b.category }}</a-tag>
                <a-tag v-for="t in b.tags" :key="t" size="small">#{{ t }}</a-tag>
                <span class="bm-visits">{{ b.visit_count }} 次访问</span>
              </div>
            </div>
            <div class="bm-ops">
              <button class="op-btn" title="打开" @click.stop="openUrl(b)"><GlobalOutlined /></button>
              <button class="op-btn" title="收藏" @click.stop="toggleFav(b)">
                <StarFilled v-if="b.is_favorited" /><StarOutlined v-else />
              </button>
              <button class="op-btn" title="归档/取消归档" @click.stop="toggleArchive(b)"><InboxOutlined /></button>
              <button class="op-btn" title="编辑" @click.stop="openEdit(b)"><EditOutlined /></button>
              <button class="op-btn danger" title="删除" @click.stop="remove(b)"><DeleteOutlined /></button>
            </div>
          </div>
        </div>

        <!-- 卡片视图 -->
        <div v-else class="bm-grid">
          <div
            v-for="b in filtered"
            :key="b.short_id"
            class="bm-card"
            :class="{ archived: b.is_archived }"
            @click="openUrl(b)"
          >
            <div class="bm-card-head">
              <img class="bm-card-fav" :src="faviconUrl(b)" @error="(e: any) => (e.target.style.visibility = 'hidden')" />
              <div class="bm-card-marks">
                <StarFilled v-if="b.is_favorited" class="bm-fav-mark" />
                <InboxOutlined v-if="b.is_archived" class="bm-arch-mark" />
              </div>
            </div>
            <div class="bm-card-title">{{ b.title }}</div>
            <div class="bm-card-url">{{ host(b.url) }}</div>
            <div class="bm-card-tags">
              <a-tag v-if="b.category" size="small" color="geekblue">{{ b.category }}</a-tag>
              <a-tag v-for="t in b.tags" :key="t" size="small">#{{ t }}</a-tag>
            </div>
            <div class="bm-card-ops" @click.stop>
              <button class="op-btn" title="打开" @click="openUrl(b)"><GlobalOutlined /></button>
              <button class="op-btn" title="收藏" @click="toggleFav(b)">
                <StarFilled v-if="b.is_favorited" /><StarOutlined v-else />
              </button>
              <button class="op-btn" title="编辑" @click="openEdit(b)"><EditOutlined /></button>
              <button class="op-btn danger" title="删除" @click="remove(b)"><DeleteOutlined /></button>
            </div>
          </div>
        </div>
      </div>
    </section>

    <a-drawer
      :open="drawerOpen"
      :title="editing ? '编辑书签' : '新增书签'"
      width="420"
      @close="drawerOpen = false"
    >
      <a-form layout="vertical">
        <a-form-item label="网址" required>
          <a-input v-model:value="form.url" placeholder="https://…" />
        </a-form-item>
        <a-form-item label="标题">
          <a-input v-model:value="form.title" placeholder="留空则用网址" />
        </a-form-item>
        <a-form-item label="描述">
          <a-textarea v-model:value="form.description" :rows="2" />
        </a-form-item>
        <a-form-item label="分类">
          <a-auto-complete
            v-model:value="form.category"
            :options="categories.map((c) => ({ value: c.name }))"
            placeholder="选择或输入"
            allow-clear
          />
        </a-form-item>
        <a-form-item label="标签">
          <a-select
            v-model:value="form.tags"
            mode="tags"
            :options="allTags.map((t) => ({ value: t, label: t }))"
            placeholder="输入后回车"
            :token-separators="[',']"
          />
        </a-form-item>
      </a-form>
      <template #footer>
        <div class="drawer-footer">
          <a-button @click="drawerOpen = false">取消</a-button>
          <a-button type="primary" :loading="saving" @click="save">保存</a-button>
        </div>
      </template>
    </a-drawer>
  </div>
</template>

<style scoped>
.bm-view {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  overflow: hidden;
  min-height: 0;
}
.panel-head {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: 10px 14px;
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-elevated, var(--bg-surface));
}
.head-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}
.head-count {
  font-size: 12px;
  color: var(--text-tertiary);
  background: var(--bg-input);
  padding: 1px 8px;
  border-radius: 10px;
}
.head-search {
  position: relative;
  margin-left: auto;
}
.head-search .search-icon {
  position: absolute;
  left: 8px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-tertiary);
  font-size: 12px;
}
.head-search input {
  height: 28px;
  width: 200px;
  padding: 0 10px 0 26px;
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 12px;
  outline: none;
}
.head-search input:focus {
  border-color: var(--color-primary-border);
}
/* 视图切换 */
.view-toggle {
  display: inline-flex;
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  overflow: hidden;
}
.vt-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: var(--bg-surface);
  color: var(--text-tertiary);
  cursor: pointer;
  font-size: 13px;
  transition: all var(--transition-fast);
}
.vt-btn:hover {
  color: var(--text-primary);
}
.vt-btn.on {
  background: var(--color-primary-bg);
  color: var(--color-primary);
}
.head-actions {
  display: flex;
  gap: 4px;
}
.icon-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--border-base);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.icon-action:hover {
  border-color: var(--color-primary-border);
  color: var(--color-primary);
}
.icon-action.primary {
  border-color: var(--color-primary);
  background: var(--color-primary);
  color: #fff;
}
.icon-action.primary:hover {
  background: var(--color-primary-hover);
  border-color: var(--color-primary-hover);
  color: #fff;
}

.panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}
.bm-empty {
  text-align: center;
  color: var(--text-tertiary);
  padding: 60px 0;
  font-size: 13px;
}
.bm-fav-mark {
  color: var(--color-warning, #faad14);
  font-size: 11px;
}
.bm-arch-mark {
  color: var(--text-tertiary);
  font-size: 11px;
}

/* ============ 列表视图 ============ */
.bm-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  transition: background var(--transition-fast);
}
.bm-item:hover {
  background: var(--bg-input);
}
.bm-item.archived {
  opacity: 0.55;
}
.bm-fav {
  width: 20px;
  height: 20px;
  border-radius: 4px;
  flex-shrink: 0;
}
.bm-main {
  flex: 1;
  min-width: 0;
  cursor: pointer;
}
.bm-title-line {
  display: flex;
  align-items: center;
  gap: 6px;
}
.bm-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.bm-url {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}
.bm-tags {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 4px;
}
.bm-visits {
  font-size: 10px;
  color: var(--text-tertiary);
  margin-left: 4px;
}
.bm-ops {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity var(--transition-fast);
}
.bm-item:hover .bm-ops {
  opacity: 1;
}

/* ============ 卡片视图 ============ */
.bm-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: var(--spacing-sm);
  padding: 4px;
}
.bm-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  background: var(--bg-elevated, var(--bg-surface));
  cursor: pointer;
  transition: border-color var(--transition-fast), transform var(--transition-fast);
}
.bm-card:hover {
  border-color: var(--color-primary-border);
  transform: translateY(-2px);
}
.bm-card.archived {
  opacity: 0.55;
}
.bm-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.bm-card-fav {
  width: 24px;
  height: 24px;
  border-radius: 6px;
}
.bm-card-marks {
  display: inline-flex;
  gap: 4px;
}
.bm-card-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.bm-card-url {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.bm-card-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  min-height: 22px;
}
.bm-card-ops {
  display: flex;
  gap: 2px;
  margin-top: 4px;
  padding-top: 6px;
  border-top: 1px solid var(--border-light);
  opacity: 0;
  transition: opacity var(--transition-fast);
}
.bm-card:hover .bm-card-ops {
  opacity: 1;
}

/* ============ 操作按钮（两种视图共用） ============ */
.op-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  border-radius: var(--radius-xs);
  cursor: pointer;
  font-size: 13px;
  transition: all var(--transition-fast);
}
.op-btn:hover {
  background: var(--bg-elevated, var(--bg-surface));
  color: var(--text-primary);
}
.op-btn.danger:hover {
  color: var(--color-error, #ff4d4f);
}
.drawer-footer {
  text-align: right;
}
.drawer-footer button {
  margin-left: 8px;
}
</style>
