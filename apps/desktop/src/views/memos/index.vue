<script setup lang="ts">
/**
 * 备忘录页 /memos（byte-memos 核心领域）
 *
 * 列表 / 卡片两种视图（工具栏切换，localStorage 持久化）
 * + 新增/编辑抽屉（md-editor-v3 Markdown 编辑器：标题/内容/分类/标签/可见性/置顶）
 * + 本地实时搜索 + 分类/标签筛选 + 置顶切换 + 删除（软删进回收站）。
 * 支持 ?id= 路由参数直接打开编辑（置顶页/标签页/仪表盘/全局搜索跳转目标）。
 */
import { ref, computed, onMounted, defineAsyncComponent } from "vue";
import { useRoute } from "vue-router";
import {
  PlusOutlined,
  ReloadOutlined,
  PushpinFilled,
  PushpinOutlined,
  EditOutlined,
  DeleteOutlined,
  EyeOutlined,
  LockOutlined,
  SearchOutlined,
  AppstoreOutlined,
  UnorderedListOutlined,
  HistoryOutlined,
  DownloadOutlined,
  CalendarOutlined,
  FieldTimeOutlined,
  PaperClipOutlined,
  UploadOutlined,
} from "@ant-design/icons-vue";
import { message, Modal } from "ant-design-vue";
import "md-editor-v3/lib/style.css";
import { api } from "@/api";
import type { AttachmentInfo, Category, DailyCount, Memo, MemoVersionInfo } from "@/api/types";
import { attachmentObjectUrl, revokeAttachmentUrl, isImageMime, fmtSize } from "@/api/attachments";
import { useThemeStore } from "@/stores/theme";
import { useLayoutStore } from "@/stores/layout";
import MemoCalendar from "./components/MemoCalendar.vue";
import MemoTimeline from "./components/MemoTimeline.vue";

/** 编辑器懒加载：1.6MB 的 md-editor-v3 只在首次打开抽屉时拉取（异步 chunk） */
const MdEditor = defineAsyncComponent(
  () => import("md-editor-v3").then((m) => m.MdEditor),
);

const layout = useLayoutStore();
const themeStore = useThemeStore();
const route = useRoute();

const memos = ref<Memo[]>([]);
const categories = ref<Category[]>([]);
const allTags = ref<string[]>([]);
const loading = ref(false);

const searchQuery = ref("");
const filterCat = ref<string>("");
const filterTag = ref<string>("");

/** 视图模式：list（行）/ card（网格）/ timeline（时间轴）/ calendar（日历），localStorage 持久化 */
const viewMode = ref<"list" | "card" | "timeline" | "calendar">(
  (localStorage.getItem("memo-view-mode") as "list" | "card" | "timeline" | "calendar") || "list",
);
function setView(m: "list" | "card" | "timeline" | "calendar") {
  viewMode.value = m;
  localStorage.setItem("memo-view-mode", m);
}

/** 日历选中日期（YYYY-MM-DD 本地日期；null=不限）——对所有视图生效的可关闭筛选 */
const filterDate = ref<string | null>(null);

/** 日历热力数据：本地日期 → 当日条数 */
const dailyCounts = ref<Record<string, number>>({});
async function loadDailyCounts() {
  try {
    const rows: DailyCount[] = await api.dailyCounts();
    const rec: Record<string, number> = {};
    for (const r of rows) rec[r.date] = r.count;
    dailyCounts.value = rec;
  } catch {
    dailyCounts.value = {};
  }
}

/** ISO(UTC) → 本地日期键（与 dailyCounts 的 'localtime' 聚焦口径一致） */
function localDateKey(iso: string): string {
  const d = new Date(iso);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** 附件索引：memo short_id → 附件列表（徽标用，一次全量查询） */
const attachMap = ref<Map<string, AttachmentInfo[]>>(new Map());
async function loadAttachMap() {
  try {
    const all = await api.attachmentListAll();
    const map = new Map<string, AttachmentInfo[]>();
    for (const a of all) {
      if (!map.has(a.memo_short_id)) map.set(a.memo_short_id, []);
      map.get(a.memo_short_id)!.push(a);
    }
    attachMap.value = map;
  } catch {
    attachMap.value = new Map();
  }
}
function attachCount(m: Memo): number {
  return attachMap.value.get(m.short_id)?.length || 0;
}

// 新增/编辑抽屉
const drawerOpen = ref(false);
const editing = ref<Memo | null>(null);
const saving = ref(false);
const form = ref({
  title: "",
  content: "",
  category: undefined as string | undefined,
  tags: [] as string[],
  visibility: "PRIVATE",
  pinned: false,
});

const VISIBILITY_OPTIONS = [
  { value: "PRIVATE", label: "私有" },
  { value: "PROTECTED", label: "受保护" },
  { value: "PUBLIC", label: "公开" },
];

/** 本地实时过滤（已加载列表内：标题/内容/标签/日期） */
const filtered = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  let list = memos.value;
  if (filterCat.value) list = list.filter((m) => m.category === filterCat.value);
  if (filterTag.value) list = list.filter((m) => m.tags.includes(filterTag.value));
  if (filterDate.value) list = list.filter((m) => localDateKey(m.created_at) === filterDate.value);
  if (!q) return list;
  return list.filter(
    (m) =>
      m.title.toLowerCase().includes(q) ||
      m.content.toLowerCase().includes(q) ||
      m.tags.some((t) => t.toLowerCase().includes(q)),
  );
});

/** 分类名 → 主题色映射（个性化分类色，null 则前端回落默认 geekblue 标签） */
const catColorMap = computed(() => {
  const m = new Map<string, string>();
  for (const c of categories.value) if (c.color) m.set(c.name, c.color);
  return m;
});

/** Markdown → 纯文本摘要（列表/卡片预览用） */
function excerpt(m: Memo, max = 140): string {
  const text = m.content
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/[#>*_~`\-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return text.length > max ? `${text.slice(0, max)}…` : text;
}

function fmtDate(iso: string): string {
  if (!iso) return "";
  return iso.replace("T", " ").slice(0, 16);
}

async function load() {
  loading.value = true;
  try {
    // 支持 ?cat= 路由参数（分类页跳转）作为初始筛选
    const cat = route.query.cat as string | undefined;
    if (cat && !filterCat.value) {
      filterCat.value = cat;
    }
    memos.value = await api.listMemos({
      category: filterCat.value || undefined,
      tag: filterTag.value || undefined,
      limit: 500,
    });
    try {
      categories.value = await api.listCategories();
    } catch {
      categories.value = [];
    }
    loadDailyCounts();
    loadAttachMap();
    updateFooter();
    // 支持 ?id= 路由参数（置顶页/标签页/仪表盘/全局搜索跳转）→ 打开编辑
    const id = route.query.id as string | undefined;
    if (id) {
      try {
        openEdit(await api.getMemo(id));
      } catch {
        /* 忽略：可能已被删除 */
      }
    }
    // 支持 ?new（仪表盘 hero CTA）→ 直接打开新建抽屉
    else if (route.query.new !== undefined) {
      openAdd();
    }
  } catch (e: any) {
    message.error(typeof e === "string" ? e : "加载失败");
  } finally {
    loading.value = false;
  }
}

function updateFooter() {
  layout.setStatusItems([
    { key: "view", text: "备忘录" },
    { key: "count", text: `共 ${memos.value.length} 篇` },
  ]);
}

function patchInList(u: Memo) {
  const idx = memos.value.findIndex((m) => m.short_id === u.short_id);
  if (idx >= 0) memos.value = memos.value.map((m, i) => (i === idx ? u : m));
}

async function togglePin(m: Memo) {
  try {
    patchInList(await api.toggleMemoPinned(m.short_id));
  } catch {
    message.error("操作失败");
  }
}

function remove(m: Memo) {
  Modal.confirm({
    title: "移入回收站",
    content: `确认将「${m.title || "无标题备忘录"}」移入回收站？可在回收站恢复。`,
    okType: "danger",
    onOk: async () => {
      await api.deleteMemo(m.short_id);
      message.success("已移入回收站");
      memos.value = memos.value.filter((x) => x.short_id !== m.short_id);
      updateFooter();
    },
  });
}

// ===== 版本历史 =====

const historyOpen = ref(false);
const historyLoading = ref(false);
const historyTarget = ref<Memo | null>(null);
const historyList = ref<MemoVersionInfo[]>([]);

function fmtDateTime(iso: string): string {
  return iso ? iso.replace("T", " ").slice(0, 16) : "";
}

async function openHistory(m: Memo) {
  historyTarget.value = m;
  historyOpen.value = true;
  historyLoading.value = true;
  try {
    historyList.value = await api.memoHistory(m.short_id);
  } catch {
    historyList.value = [];
  } finally {
    historyLoading.value = false;
  }
}

/** 回滚：当前内容会先自动存为新版本（update_memo 内建快照），不丢数据 */
function restoreVersion(v: MemoVersionInfo) {
  const target = historyTarget.value;
  if (!target) return;
  Modal.confirm({
    title: "回滚到此版本",
    content: `将「${target.title || "(无标题)"}」内容回滚到 ${fmtDateTime(v.created_at)} 的版本。当前内容会先存为新版本，不会丢失。`,
    okText: "回滚",
    onOk: async () => {
      try {
        await api.restoreMemoVersion(target.short_id, v.id);
        message.success("已回滚");
        historyList.value = await api.memoHistory(target.short_id);
        await load();
      } catch (e: any) {
        message.error(typeof e === "string" ? e : "回滚失败");
      }
    },
  });
}

// ===== 单篇导出 =====

async function exportMemo(m: Memo, format: "markdown" | "html") {
  const ext = format === "html" ? "html" : "md";
  const base = (m.title || "memo").replace(/[\\/:*?"<>|]/g, "_").slice(0, 60) || "memo";
  const path = await api.pickExportPath(`${base}.${ext}`, ext);
  if (!path) return; // 用户取消
  try {
    const saved = await api.exportMemo(m.short_id, format, path);
    message.success(`已导出：${saved}`);
  } catch (e: any) {
    message.error(typeof e === "string" ? e : "导出失败");
  }
}

async function loadTags() {
  try {
    allTags.value = (await api.tagStats(200)).map((t) => t.tag);
  } catch {
    allTags.value = [];
  }
}

function openAdd() {
  editing.value = null;
  form.value = {
    title: "",
    content: "",
    category: filterCat.value || undefined,
    tags: filterTag.value ? [filterTag.value] : [],
    visibility: "PRIVATE",
    pinned: false,
  };
  loadTags();
  drawerOpen.value = true;
}

function openEdit(m: Memo) {
  editing.value = m;
  form.value = {
    title: m.title,
    content: m.content,
    category: m.category ?? undefined,
    tags: [...m.tags],
    visibility: m.visibility,
    pinned: m.pinned,
  };
  loadTags();
  loadEditAttaches(m.short_id);
  drawerOpen.value = true;
}

// ===== 编辑抽屉附件 =====

const editAttaches = ref<AttachmentInfo[]>([]);
const editAttUrls = ref<Record<number, string>>({});
const uploading = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

async function loadEditAttaches(shortId: string) {
  try {
    editAttaches.value = await api.attachmentList(shortId);
  } catch {
    editAttaches.value = [];
  }
  // 图片附件预取缩略 ObjectURL
  const urls: Record<number, string> = {};
  for (const a of editAttaches.value) {
    if (isImageMime(a.mime_type)) {
      try {
        urls[a.id] = await attachmentObjectUrl(a.id);
      } catch {
        /* 预览失败不阻断 */
      }
    }
  }
  editAttUrls.value = urls;
}

async function onPickFiles(e: Event) {
  const input = e.target as HTMLInputElement;
  const files = Array.from(input.files || []);
  input.value = "";
  if (!files.length || !editing.value) return;
  uploading.value = true;
  try {
    for (const f of files) {
      const bytes = new Uint8Array(await f.arrayBuffer());
      await api.attachmentUpload(editing.value.short_id, f.name, f.type || "", bytes);
    }
    message.success(`已添加 ${files.length} 个附件`);
    await loadEditAttaches(editing.value.short_id);
    loadAttachMap();
  } catch (err: any) {
    message.error(typeof err === "string" ? err : "附件上传失败");
  } finally {
    uploading.value = false;
  }
}

async function removeAttachment(a: AttachmentInfo) {
  if (!editing.value) return;
  try {
    await api.attachmentDelete(a.id);
    revokeAttachmentUrl(a.id);
    message.success("附件已删除");
    await loadEditAttaches(editing.value.short_id);
    loadAttachMap();
  } catch {
    message.error("删除失败");
  }
}

async function downloadAttachment(a: AttachmentInfo) {
  const ext = a.filename.includes(".") ? a.filename.split(".").pop()! : "bin";
  const path = await api.pickExportPath(a.filename, ext);
  if (!path) return;
  try {
    const saved = await api.attachmentSave(a.id, path);
    message.success(`已保存：${saved}`);
  } catch {
    message.error("保存失败");
  }
}

async function save() {
  if (!form.value.title.trim() && !form.value.content.trim()) {
    message.warning("标题和内容不能同时为空");
    return;
  }
  saving.value = true;
  try {
    if (editing.value) {
      const u = await api.updateMemo(editing.value.short_id, {
        title: form.value.title.trim() || "无标题",
        content: form.value.content,
        visibility: form.value.visibility,
        category: form.value.category ?? null,
        tags: form.value.tags,
        pinned: form.value.pinned,
      });
      patchInList(u);
      message.success("已保存");
    } else {
      const created = await api.addMemo({
        title: form.value.title.trim() || "无标题",
        content: form.value.content,
        visibility: form.value.visibility,
        category: form.value.category ?? null,
        tags: form.value.tags,
      });
      memos.value = [created, ...memos.value];
      message.success("已添加");
    }
    drawerOpen.value = false;
    try {
      categories.value = await api.listCategories();
    } catch {
      /* 忽略 */
    }
    loadDailyCounts();
    loadAttachMap();
    updateFooter();
  } catch (e: any) {
    message.error(typeof e === "string" ? e : "保存失败");
  } finally {
    saving.value = false;
  }
}

onMounted(load);
</script>

<template>
  <div class="memo-view">
    <section class="panel">
      <header class="panel-head">
        <span class="head-title">备忘录</span>
        <span class="head-count">{{ filtered.length }}/{{ memos.length }}</span>

        <div class="head-search">
          <SearchOutlined class="search-icon" />
          <input v-model="searchQuery" placeholder="搜索备忘录…" />
        </div>

        <a-select
          v-model:value="filterCat"
          size="small"
          style="width: 130px"
          placeholder="全部分类"
          allow-clear
          @change="load"
        >
          <a-select-option value="">全部分类</a-select-option>
          <a-select-option v-for="c in categories" :key="c.id" :value="c.name">
            {{ c.name }}（{{ c.memo_count }}）
          </a-select-option>
        </a-select>

        <a-select
          v-model:value="filterTag"
          size="small"
          style="width: 120px"
          placeholder="全部标签"
          allow-clear
          @change="load"
        >
          <a-select-option value="">全部标签</a-select-option>
          <a-select-option v-for="t in allTags" :key="t" :value="t">#{{ t }}</a-select-option>
        </a-select>

        <a-tag v-if="filterDate" class="date-chip" closable @close="filterDate = null">
          <CalendarOutlined /> {{ filterDate }}
        </a-tag>

        <div class="view-toggle">
          <button class="vt-btn" :class="{ on: viewMode === 'list' }" title="列表视图" @click="setView('list')">
            <UnorderedListOutlined />
          </button>
          <button class="vt-btn" :class="{ on: viewMode === 'card' }" title="卡片视图" @click="setView('card')">
            <AppstoreOutlined />
          </button>
          <button class="vt-btn" :class="{ on: viewMode === 'timeline' }" title="时间轴视图" @click="setView('timeline')">
            <FieldTimeOutlined />
          </button>
          <button class="vt-btn" :class="{ on: viewMode === 'calendar' }" title="日历视图" @click="setView('calendar')">
            <CalendarOutlined />
          </button>
        </div>

        <div class="head-actions">
          <button class="icon-action" title="刷新" @click="load"><ReloadOutlined /></button>
          <button class="icon-action primary" title="新增备忘录" @click="openAdd"><PlusOutlined /></button>
        </div>
      </header>

      <div class="panel-body">
        <div v-if="loading" class="memo-empty">加载中…</div>
        <div v-else-if="!filtered.length" class="memo-empty">
          {{ searchQuery || filterCat || filterTag || filterDate ? "无匹配备忘录" : "暂无备忘录 — 点 + 新建第一篇" }}
        </div>

        <!-- 日历视图：月历热力 + 当日列表 -->
        <div v-else-if="viewMode === 'calendar'" class="cal-layout">
          <div class="cal-side">
            <MemoCalendar :counts="dailyCounts" :selected="filterDate" @select="filterDate = $event" />
          </div>
          <div class="cal-day-panel">
            <div class="cal-day-title">
              {{ filterDate ? `${filterDate} · ${filtered.length} 条` : "未选择日期 — 点左侧日历" }}
            </div>
            <div v-if="!filterDate" class="cal-day-empty">选中日期后在此查看当日备忘录</div>
            <div v-else class="cal-day-list">
              <div v-for="m in filtered" :key="m.short_id" class="cal-day-item" @click="openEdit(m)">
                <span class="cal-item-time">{{ fmtDate(m.created_at).slice(11, 16) }}</span>
                <div class="cal-item-main">
                  <span class="cal-item-title">{{ m.title || "无标题" }}</span>
                  <span class="cal-item-excerpt">{{ excerpt(m, 80) }}</span>
                </div>
                <a-tag v-if="m.category" :color="catColorMap.get(m.category) || 'geekblue'" class="cal-item-cat">
                  {{ m.category }}
                </a-tag>
              </div>
            </div>
          </div>
        </div>

        <!-- 时间轴视图：日期分组纵向轨道 -->
        <div v-else-if="viewMode === 'timeline'" class="tl-wrap">
          <MemoTimeline
            :memos="filtered"
            :cat-color-map="catColorMap"
            @open="openEdit"
            @pin="togglePin"
            @del="remove"
          />
        </div>

        <!-- 列表视图 -->
        <div v-else-if="viewMode === 'list'" class="memo-list">
          <div v-for="m in filtered" :key="m.short_id" class="memo-item" @click="openEdit(m)">
            <div class="memo-main">
              <div class="memo-title-line">
                <PushpinFilled v-if="m.pinned" class="memo-pin-mark" />
                <span class="memo-title">{{ m.title || "无标题" }}</span>
                <EyeOutlined v-if="m.visibility === 'PUBLIC'" class="memo-vis-mark" title="公开" />
                <LockOutlined v-else-if="m.visibility === 'PROTECTED'" class="memo-vis-mark" title="受保护" />
              </div>
              <div class="memo-excerpt">{{ excerpt(m) }}</div>
              <div class="memo-meta">
                <a-tag v-if="m.category" :color="catColorMap.get(m.category) || 'geekblue'">{{ m.category }}</a-tag>
                <a-tag v-for="t in m.tags" :key="t">#{{ t }}</a-tag>
                <span v-if="attachCount(m)" class="attach-badge" title="附件数">
                  <PaperClipOutlined /> {{ attachCount(m) }}
                </span>
                <span class="memo-time">更新于 {{ fmtDate(m.updated_at) }}</span>
              </div>
            </div>
            <div class="memo-ops">
              <button class="op-btn" :class="{ on: m.pinned }" title="置顶/取消置顶" @click.stop="togglePin(m)">
                <PushpinFilled v-if="m.pinned" /><PushpinOutlined v-else />
              </button>
              <button class="op-btn" title="编辑" @click.stop="openEdit(m)"><EditOutlined /></button>
              <button class="op-btn" title="版本历史" @click.stop="openHistory(m)"><HistoryOutlined /></button>
              <a-dropdown :trigger="['click']">
                <button class="op-btn" title="导出" @click.stop><DownloadOutlined /></button>
                <template #overlay>
                  <a-menu @click="({ key }: any) => exportMemo(m, key as 'markdown' | 'html')">
                    <a-menu-item key="markdown">Markdown（.md）</a-menu-item>
                    <a-menu-item key="html">HTML（.html）</a-menu-item>
                  </a-menu>
                </template>
              </a-dropdown>
              <button class="op-btn danger" title="删除" @click.stop="remove(m)"><DeleteOutlined /></button>
            </div>
          </div>
        </div>

        <!-- 卡片视图 -->
        <div v-else class="memo-grid">
          <div v-for="m in filtered" :key="m.short_id" class="memo-card" @click="openEdit(m)">
            <div class="memo-card-head">
              <span class="memo-card-title">{{ m.title || "无标题" }}</span>
              <PushpinFilled v-if="m.pinned" class="memo-pin-mark" />
            </div>
            <div class="memo-card-excerpt">{{ excerpt(m, 120) }}</div>
            <div class="memo-card-meta">
              <a-tag v-if="m.category" :color="catColorMap.get(m.category) || 'geekblue'">{{ m.category }}</a-tag>
              <a-tag v-for="t in m.tags.slice(0, 3)" :key="t">#{{ t }}</a-tag>
              <span v-if="attachCount(m)" class="attach-badge" title="附件数">
                <PaperClipOutlined /> {{ attachCount(m) }}
              </span>
              <span class="memo-time">{{ fmtDate(m.updated_at) }}</span>
            </div>
            <div class="memo-ops memo-card-ops">
              <button class="op-btn" :class="{ on: m.pinned }" title="置顶/取消置顶" @click.stop="togglePin(m)">
                <PushpinFilled v-if="m.pinned" /><PushpinOutlined v-else />
              </button>
              <button class="op-btn" title="编辑" @click.stop="openEdit(m)"><EditOutlined /></button>
              <button class="op-btn" title="版本历史" @click.stop="openHistory(m)"><HistoryOutlined /></button>
              <a-dropdown :trigger="['click']">
                <button class="op-btn" title="导出" @click.stop><DownloadOutlined /></button>
                <template #overlay>
                  <a-menu @click="({ key }: any) => exportMemo(m, key as 'markdown' | 'html')">
                    <a-menu-item key="markdown">Markdown（.md）</a-menu-item>
                    <a-menu-item key="html">HTML（.html）</a-menu-item>
                  </a-menu>
                </template>
              </a-dropdown>
              <button class="op-btn danger" title="删除" @click.stop="remove(m)"><DeleteOutlined /></button>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 版本历史抽屉 -->
    <a-drawer
      v-model:open="historyOpen"
      :title="`版本历史${historyTarget ? ' · ' + (historyTarget.title || '(无标题)') : ''}`"
      placement="right"
      :width="520"
    >
      <div v-if="historyLoading" class="hist-loading">加载中…</div>
      <div v-else-if="!historyList.length" class="hist-empty">
        暂无历史版本——编辑保存时自动记录每次内容变更。
      </div>
      <div v-else class="hist-list">
        <div v-for="v in historyList" :key="v.id" class="hist-item">
          <div class="hist-head">
            <span class="hist-time">{{ fmtDateTime(v.created_at) }}</span>
            <button class="hist-restore" title="回滚到此版本" @click="restoreVersion(v)">
              <HistoryOutlined /> 恢复
            </button>
          </div>
          <pre class="hist-content">{{ v.content.length > 400 ? v.content.slice(0, 400) + "…" : v.content }}</pre>
        </div>
      </div>
    </a-drawer>

    <!-- 新增/编辑抽屉 -->
    <a-drawer
      v-model:open="drawerOpen"
      :title="editing ? '编辑备忘录' : '新增备忘录'"
      placement="right"
      :width="720"
      :destroy-on-close="true"
    >
      <div class="memo-form">
        <div class="form-row">
          <input v-model="form.title" class="form-title-input" placeholder="标题…" maxlength="200" />
        </div>

        <div class="form-row editor-row">
          <MdEditor
            v-model="form.content"
            :theme="themeStore.isDark ? 'dark' : 'light'"
            language="zh-CN"
            style="height: 420px"
            placeholder="正文（Markdown）…"
            :toolbars="['bold', 'underline', 'italic', '-', 'title', 'strikeThrough', 'sub', 'sup', 'quote', 'unorderedList', 'orderedList', 'task', '-', 'codeRow', 'code', 'link', 'image', 'table', 'mermaid', '-', 'revoke', 'next', 'save', '=', 'pageFullscreen', 'fullscreen', 'preview', 'htmlPreview', 'catalog', 'github']"
            @save="save"
          />
        </div>

        <div class="form-row form-meta-row">
          <a-auto-complete
            v-model:value="form.category"
            style="width: 180px"
            placeholder="分类（可新建）"
            allow-clear
            :options="categories.map((c) => ({ value: c.name }))"
          />
          <a-select
            v-model:value="form.tags"
            mode="tags"
            style="min-width: 220px; flex: 1"
            placeholder="标签（回车确认，支持 a/b 层级）"
            :options="allTags.map((t) => ({ value: t }))"
          />
        </div>

        <div class="form-row form-meta-row">
          <a-select v-model:value="form.visibility" style="width: 140px" :options="VISIBILITY_OPTIONS" />
          <span class="form-pin-label">置顶</span>
          <a-switch v-model:checked="form.pinned" size="small" />
        </div>

        <!-- 附件（编辑已有备忘录时可用） -->
        <div v-if="editing" class="form-row attach-section">
          <div class="attach-head">
            <span class="attach-title">附件（{{ editAttaches.length }}）</span>
            <button class="attach-upload" :disabled="uploading" @click="fileInput?.click()">
              <UploadOutlined /> {{ uploading ? "上传中…" : "添加附件" }}
            </button>
            <input ref="fileInput" type="file" multiple style="display: none" @change="onPickFiles" />
          </div>
          <div v-if="editAttaches.length" class="attach-list">
            <div v-for="a in editAttaches" :key="a.id" class="attach-item">
              <img v-if="editAttUrls[a.id]" :src="editAttUrls[a.id]" class="attach-thumb" :alt="a.filename" />
              <span v-else class="attach-icon"><PaperClipOutlined /></span>
              <span class="attach-name" :title="a.filename">{{ a.filename }}</span>
              <span class="attach-size">{{ fmtSize(a.file_size) }}</span>
              <button class="op-btn" title="下载" @click="downloadAttachment(a)"><DownloadOutlined /></button>
              <button class="op-btn danger" title="删除" @click="removeAttachment(a)"><DeleteOutlined /></button>
            </div>
          </div>
          <div v-else class="attach-empty">暂无附件 — 图片将显示缩略预览</div>
        </div>
        <div v-else class="attach-hint">附件在保存后可添加（新建完成即出现附件区）</div>

        <div class="form-actions">
          <a-button @click="drawerOpen = false">取消</a-button>
          <a-button type="primary" :loading="saving" @click="save">
            {{ editing ? "保存" : "添加" }}
          </a-button>
        </div>
      </div>
    </a-drawer>
  </div>
</template>

<style scoped>
/* ===== 页面容器 ===== */
.memo-view {
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

/* ===== 工具栏 ===== */
.panel-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-light);
  flex-shrink: 0;
  flex-wrap: wrap;
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

.head-search {
  position: relative;
  flex: 0 1 220px;
  min-width: 140px;
}
.head-search .search-icon {
  position: absolute;
  left: 9px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 12px;
  color: var(--text-tertiary);
  pointer-events: none;
}
.head-search input {
  width: 100%;
  height: 28px;
  padding: 0 10px 0 26px;
  font-size: 12px;
  color: var(--text-primary);
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-sm);
  outline: none;
  transition: border-color var(--transition-fast);
}
.head-search input:focus {
  border-color: var(--color-primary);
}

.view-toggle {
  display: flex;
  gap: 2px;
  background: var(--bg-input);
  border-radius: var(--radius-sm);
  padding: 2px;
}
.vt-btn {
  display: grid;
  place-items: center;
  width: 26px;
  height: 24px;
  font-size: 13px;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.vt-btn:hover {
  color: var(--text-primary);
}
.vt-btn.on {
  color: var(--color-primary);
  background: var(--bg-surface);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
}

.head-actions {
  display: flex;
  gap: 6px;
  margin-left: auto;
}
.icon-action {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  font-size: 14px;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.icon-action:hover {
  color: var(--text-primary);
  border-color: var(--text-tertiary);
}
.icon-action.primary {
  color: #fff;
  background: var(--color-primary);
  border-color: var(--color-primary);
}
.icon-action.primary:hover {
  opacity: 0.85;
}

/* ===== 列表 ===== */
.panel-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 8px;
}

.memo-empty {
  display: grid;
  place-items: center;
  height: 200px;
  color: var(--text-tertiary);
  font-size: 13px;
}

.memo-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.memo-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background var(--transition-fast);
}
.memo-item:hover {
  background: var(--color-hover);
}

.memo-main {
  flex: 1;
  min-width: 0;
}

.memo-title-line {
  display: flex;
  align-items: center;
  gap: 6px;
}
.memo-title {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.memo-pin-mark {
  font-size: 12px;
  color: var(--color-primary);
  flex-shrink: 0;
}
.memo-vis-mark {
  font-size: 11px;
  color: var(--text-tertiary);
  flex-shrink: 0;
}

.memo-excerpt {
  margin-top: 3px;
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.memo-meta {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 5px;
  flex-wrap: wrap;
}
.memo-meta :deep(.ant-tag) {
  margin-right: 0;
  font-size: 11px;
  line-height: 18px;
  padding: 0 6px;
}
.memo-time {
  margin-left: auto;
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
}

.memo-ops {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity var(--transition-fast);
}
.memo-item:hover .memo-ops,
.memo-card:hover .memo-ops {
  opacity: 1;
}
/* 卡片视图操作行：悬浮时浮在卡片底部 */
.memo-card-ops {
  position: absolute;
  right: 8px;
  bottom: 8px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  padding: 2px;
}
.op-btn {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  font-size: 13px;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: var(--radius-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.op-btn:hover {
  color: var(--text-primary);
  background: var(--bg-input);
}
.op-btn.on {
  color: var(--color-primary);
}
.op-btn.danger:hover {
  color: #ef4444;
}

/* ===== 卡片 ===== */
.memo-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 10px;
}

.memo-card {
  position: relative; /* 卡片内操作行绝对定位锚点 */
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 14px;
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
  min-height: 120px;
}
.memo-card:hover {
  border-color: var(--color-primary-border);
  transform: translateY(-1px);
}

.memo-card-head {
  display: flex;
  align-items: center;
  gap: 6px;
}
.memo-card-title {
  flex: 1;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.memo-card-excerpt {
  flex: 1;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.6;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.memo-card-meta {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
}
.memo-card-meta :deep(.ant-tag) {
  margin-right: 0;
  font-size: 11px;
  line-height: 18px;
  padding: 0 6px;
}

/* ===== 编辑抽屉表单 ===== */
.memo-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
  height: 100%;
}

.form-row {
  display: flex;
  gap: 10px;
}

.form-title-input {
  flex: 1;
  height: 36px;
  padding: 0 12px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-sm);
  outline: none;
  transition: border-color var(--transition-fast);
}
.form-title-input:focus {
  border-color: var(--color-primary);
}

.editor-row {
  flex: 1;
  min-height: 0;
}
.editor-row :deep(.md-editor) {
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.form-meta-row {
  align-items: center;
}

.form-pin-label {
  font-size: 12px;
  color: var(--text-secondary);
  margin-left: auto;
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

/* ===== 日历视图 ===== */
.cal-layout {
  display: grid;
  grid-template-columns: 340px 1fr;
  gap: 12px;
  align-items: start;
}
.cal-day-panel {
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  background: var(--bg-input);
  min-height: 200px;
  overflow: hidden;
}
.cal-day-title {
  padding: 9px 14px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-primary);
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-elevated);
}
.cal-day-empty {
  display: grid;
  place-items: center;
  height: 160px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.cal-day-list {
  display: flex;
  flex-direction: column;
}
.cal-day-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  border-bottom: 1px solid var(--border-light);
  cursor: pointer;
  transition: background var(--transition-fast);
}
.cal-day-item:last-child {
  border-bottom: none;
}
.cal-day-item:hover {
  background: var(--color-hover);
}
.cal-item-time {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
}
.cal-item-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.cal-item-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cal-item-excerpt {
  font-size: 11.5px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cal-item-cat {
  margin-right: 0;
  font-size: 10.5px;
  line-height: 17px;
  padding: 0 6px;
}

/* ===== 时间轴视图 ===== */
.tl-wrap {
  max-width: 880px;
  margin: 0 auto;
}

/* ===== 附件徽标（列表/卡片 meta） ===== */
.attach-badge {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 10.5px;
  color: var(--text-tertiary);
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: 10px;
  padding: 0 6px;
  line-height: 17px;
}

/* ===== 编辑抽屉附件区 ===== */
.attach-section {
  flex-direction: column;
  gap: 8px;
}
.attach-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.attach-title {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-secondary);
}
.attach-upload {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  padding: 0 10px;
  font-size: 12px;
  color: var(--color-primary);
  background: transparent;
  border: 1px solid var(--color-primary-border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background var(--transition-fast);
}
.attach-upload:hover {
  background: var(--color-primary-bg);
}
.attach-upload:disabled {
  opacity: 0.6;
  cursor: default;
}
.attach-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 180px;
  overflow: auto;
}
.attach-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 8px;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
}
.attach-thumb {
  width: 34px;
  height: 34px;
  object-fit: cover;
  border-radius: var(--radius-xs);
  flex-shrink: 0;
}
.attach-icon {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  font-size: 15px;
  color: var(--text-tertiary);
  background: var(--bg-input);
  border-radius: var(--radius-xs);
  flex-shrink: 0;
}
.attach-name {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.attach-size {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
}
.attach-empty,
.attach-hint {
  font-size: 11.5px;
  color: var(--text-tertiary);
  padding: 6px 2px;
}

/* ===== 版本历史抽屉 ===== */
.hist-loading,
.hist-empty {
  padding: 40px 16px;
  text-align: center;
  font-size: 12px;
  color: var(--text-tertiary);
}
.hist-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.hist-item {
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  background: var(--bg-input);
  overflow: hidden;
}
.hist-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-elevated);
}
.hist-time {
  font-size: 12px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", monospace;
}
.hist-restore {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: var(--color-primary);
  background: none;
  border: 1px solid var(--color-primary-border);
  border-radius: var(--radius-sm);
  padding: 2px 8px;
  cursor: pointer;
  transition: background var(--transition-fast);
}
.hist-restore:hover {
  background: var(--color-primary-bg);
}
.hist-content {
  margin: 0;
  padding: 10px 12px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 180px;
  overflow-y: auto;
  font-family: inherit;
}
</style>
