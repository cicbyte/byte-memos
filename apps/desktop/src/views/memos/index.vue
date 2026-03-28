<script setup lang="ts">
/**
 * 备忘录页 /memos（byte-memos 核心领域）
 *
 * 列表 / 卡片两种视图（工具栏切换，localStorage 持久化）
 * + 新增/编辑抽屉（md-editor-v3 Markdown 编辑器：标题/内容/分类/标签/可见性/置顶）
 * + 本地实时搜索 + 分类/标签筛选 + 置顶切换 + 删除（软删进回收站）。
 * 支持 ?id= 路由参数直接打开编辑（置顶页/标签页/仪表盘/全局搜索跳转目标）。
 */
import { ref, computed, onMounted, defineAsyncComponent, watch, nextTick } from "vue";
import { useRoute } from "vue-router";
import {
  PlusOutlined,
  ReloadOutlined,
  PushpinFilled,
  PushpinOutlined,
  EditOutlined,
  DeleteOutlined,
  SearchOutlined,
  AppstoreOutlined,
  HistoryOutlined,
  DownloadOutlined,
  CalendarOutlined,
  FieldTimeOutlined,
  PaperClipOutlined,
  UploadOutlined,
  ExpandOutlined,
  SendOutlined,
  BoldOutlined,
  ItalicOutlined,
  CodeOutlined,
  LinkOutlined,
  UnorderedListOutlined,
  FolderOutlined,
  CloseOutlined,
} from "@ant-design/icons-vue";
import { message, Modal } from "ant-design-vue";
import "md-editor-v3/lib/style.css";
import { api } from "@/api";
import type { AttachmentInfo, Category, DailyCount, Memo, MemoVersionInfo, TagStat } from "@/api/types";
import { useAiProviderStore } from "@/stores/aiProvider";
import { AI_ACTIONS, TRANSLATE_LANGS, findAction, parseAiTags, parseAiCategory, splitAiTitle, type AiActionDef } from "./aiActions";
import { attachmentObjectUrl, revokeAttachmentUrl, isImageMime, fmtSize } from "@/api/attachments";
import { useThemeStore } from "@/stores/theme";
import { useLayoutStore } from "@/stores/layout";
import MemoCalendar from "./components/MemoCalendar.vue";
import MemoTimeline from "./components/MemoTimeline.vue";
import AiResultModal from "./components/AiResultModal.vue";

/** 编辑器懒加载：1.6MB 的 md-editor-v3 只在首次打开抽屉时拉取（异步 chunk） */
const MdEditor = defineAsyncComponent(
  () => import("md-editor-v3").then((m) => m.MdEditor),
);

const layout = useLayoutStore();
const themeStore = useThemeStore();
const route = useRoute();

const memos = ref<Memo[]>([]);
const categories = ref<Category[]>([]);
const allTags = ref<TagStat[]>([]);
const loading = ref(false);

const searchQuery = ref("");
const filterCat = ref<string>("");
const filterTag = ref<string>("");

/** 主区流形态：timeline（默认）/ card，localStorage 持久化（兼容旧四态值） */
const viewMode = ref<"timeline" | "card">(
  localStorage.getItem("memo-view-mode") === "card" ? "card" : "timeline",
);
function setView(m: "timeline" | "card") {
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

// ===== 左栏：标签筛选 + 统计 =====

function toggleTagFilter(tag: string) {
  filterTag.value = filterTag.value === tag ? "" : tag;
}

const todayKeyStr = (() => {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
})();
const monthPrefix = todayKeyStr.slice(0, 7);

const todayCount = computed(() => dailyCounts.value[todayKeyStr] || 0);
const monthCount = computed(() => {
  let sum = 0;
  for (const [date, c] of Object.entries(dailyCounts.value)) {
    if (date.startsWith(monthPrefix)) sum += c;
  }
  return sum;
});

// ===== 顶部固定快速输入 =====

const quickText = ref("");
const savingQuick = ref(false);
const quickCat = ref<string>("");
const focused = ref(false);
const quickEl = ref<HTMLTextAreaElement | null>(null);
const titleInput = ref<HTMLInputElement | null>(null);

// ===== AI 动作 =====
const aiStore = useAiProviderStore();
const aiModal = ref<{ open: boolean; memo: Memo | null; action: AiActionDef | null }>({
  open: false,
  memo: null,
  action: null,
});

/** 打开 AI 动作弹框（先过就绪守卫：未配置供应商 → 提示 + 跳设置） */
async function openAi(m: Memo, action: string, lang = "") {
  if (!(await aiStore.ensureReady())) return;
  aiModal.value = { open: true, memo: m, action: findAction(action, lang) };
}

/** 应用 AI 结果：替换 / 追加 / 另存为新备忘录 / 采纳标签 / 采纳分类 */
async function onAiApply(payload: { mode: string; result: string; action: AiActionDef }) {
  const m = aiModal.value.memo;
  if (!m) return;
  const { mode, result, action } = payload;
  try {
    if (mode === "adoptTags") {
      const tags = [...new Set([...m.tags, ...parseAiTags(result)])];
      const updated = await api.updateMemo(m.short_id, {
        title: m.title,
        content: m.content,
        visibility: m.visibility,
        category: m.category ?? null,
        tags,
        pinned: m.pinned,
      });
      memos.value = memos.value.map((x) => (x.short_id === m.short_id ? updated : x));
      message.success("已采纳标签");
    } else if (mode === "adoptCategory") {
      const cat = parseAiCategory(result);
      if (!cat) {
        message.warning("AI 未给出匹配的分类");
        return;
      }
      const updated = await api.updateMemo(m.short_id, {
        title: m.title,
        content: m.content,
        visibility: m.visibility,
        category: cat,
        tags: m.tags,
        pinned: m.pinned,
      });
      memos.value = memos.value.map((x) => (x.short_id === m.short_id ? updated : x));
      message.success(`已归入「${cat}」`);
    } else {
      const { title: aiTitle, body } = splitAiTitle(result);
      if (mode === "replace") {
        const useAiTitle = action.action === "format" && aiTitle;
        const updated = await api.updateMemo(m.short_id, {
          title: useAiTitle ? (aiTitle as string) : m.title,
          content: useAiTitle ? body : result,
          visibility: m.visibility,
          category: m.category ?? null,
          tags: m.tags,
          pinned: m.pinned,
        });
        memos.value = memos.value.map((x) => (x.short_id === m.short_id ? updated : x));
        message.success("已替换");
      } else if (mode === "append") {
        const updated = await api.updateMemo(m.short_id, {
          title: m.title,
          content: `${m.content}\n\n${result}`,
          visibility: m.visibility,
          category: m.category ?? null,
          tags: m.tags,
          pinned: m.pinned,
        });
        memos.value = memos.value.map((x) => (x.short_id === m.short_id ? updated : x));
        message.success("已追加");
      } else if (mode === "saveNew") {
        const created = await api.addMemo({
          title: aiTitle ?? (m.title ? `${m.title} · ${action.label}` : `AI ${action.label}`),
          content: aiTitle ? body : result,
          visibility: m.visibility,
          category: m.category ?? null,
          tags: m.tags,
        });
        memos.value = [created, ...memos.value];
        message.success("已另存为新备忘录");
        updateFooter();
      }
    }
    aiModal.value = { ...aiModal.value, open: false };
  } catch (e) {
    message.error(`应用失败：${e}`);
  }
}

/** 输入自增高（1→最多 150px），提交后复位 */
function autoGrow() {
  const el = quickEl.value;
  if (!el) return;
  el.style.height = "auto";
  el.style.height = `${Math.min(el.scrollHeight, 150)}px`;
}

// ----- 快速输入草稿（防误关丢失） -----
const QUICK_DRAFT_KEY = "bm-quick-draft";
const hasQuickDraft = ref(false);
let quickDraftTimer: number | undefined;

watch(quickText, (v) => {
  window.clearTimeout(quickDraftTimer);
  quickDraftTimer = window.setTimeout(() => {
    if (v.trim()) localStorage.setItem(QUICK_DRAFT_KEY, v);
    else localStorage.removeItem(QUICK_DRAFT_KEY);
  }, 400);
});

onMounted(() => {
  const d = localStorage.getItem(QUICK_DRAFT_KEY);
  if (d && d.trim()) {
    quickText.value = d;
    hasQuickDraft.value = true;
    nextTick(autoGrow);
  }
});

function discardQuickDraft() {
  quickText.value = "";
  localStorage.removeItem(QUICK_DRAFT_KEY);
  hasQuickDraft.value = false;
}

// ----- Mini Markdown 工具栏（光标处插入/包裹） -----
function insertMd(before: string, after = "") {
  const el = quickEl.value;
  if (!el) return;
  const s = el.selectionStart ?? quickText.value.length;
  const e = el.selectionEnd ?? s;
  const sel = quickText.value.slice(s, e);
  quickText.value = quickText.value.slice(0, s) + before + sel + after + quickText.value.slice(e);
  nextTick(() => {
    el.focus();
    const pos = s + before.length + sel.length;
    el.setSelectionRange(pos, pos);
    autoGrow();
  });
}

/** 首行 ≤40 字取作标题，余下为正文；首行过长则整体作正文、截断作标题 */
function splitQuick(raw: string): { title: string; body: string } {
  const lines = raw.replace(/\r/g, "").split("\n");
  const first = (lines[0] || "").trim();
  if (first.length > 0 && first.length <= 40) {
    const rest = lines.slice(1).join("\n").trim();
    return { title: first, body: rest || first };
  }
  return { title: first.slice(0, 40) || "无标题", body: raw };
}

async function submitQuick() {
  const raw = quickText.value.trim();
  if (!raw || savingQuick.value) return;
  savingQuick.value = true;
  try {
    const { title, body } = splitQuick(raw);
    const created = await api.addMemo({
      title,
      content: body,
      visibility: "PRIVATE",
      category: quickCat.value || null,
      tags: filterTag.value ? [filterTag.value] : [],
    });
    memos.value = [created, ...memos.value];
    quickText.value = "";
    if (quickEl.value) quickEl.value.style.height = "auto";
    message.success("已记下");
    loadDailyCounts();
    updateFooter();
  } catch {
    message.error("保存失败");
  } finally {
    savingQuick.value = false;
  }
}

function onQuickKey(e: KeyboardEvent) {
  // IME 组合中不触发（中文输入法 Ctrl/Enter 属于组词流程）
  if (e.isComposing || e.keyCode === 229) return;
  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
    e.preventDefault();
    submitQuick();
  } else if (e.key === "Escape") {
    (e.target as HTMLTextAreaElement).blur();
  }
}

/** 快速输入 → 完整编辑抽屉（预填，可补分类/标签/附件） */
function expandQuick() {
  const raw = quickText.value;
  const parts = raw.trim() ? splitQuick(raw) : { title: "", body: "" };
  editing.value = null;
  form.value = {
    title: parts.title === "无标题" ? "" : parts.title,
    content: parts.body,
    category: quickCat.value || filterCat.value || undefined,
    tags: filterTag.value ? [filterTag.value] : [],
    visibility: "PRIVATE",
    pinned: false,
  };
  loadTags();
  drawerOpen.value = true;
}

function clearFilters() {
  searchQuery.value = "";
  filterCat.value = "";
  filterTag.value = "";
  filterDate.value = null;
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

// 弹框打开时自动聚焦标题
watch(drawerOpen, (v) => {
  if (v) nextTick(() => titleInput.value?.focus());
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
    allTags.value = await api.tagStats(200);
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
  // 恢复上次未完成的新建草稿
  try {
    const raw = localStorage.getItem(EDITOR_DRAFT_KEY);
    if (raw) {
      const p = JSON.parse(raw) as { t?: string; c?: string };
      if ((p.t && p.t.trim()) || (p.c && p.c.trim())) {
        form.value.title = p.t || "";
        form.value.content = p.c || "";
        message.info("已恢复上次未完成的草稿");
      }
    }
  } catch {
    /* 草稿损坏则忽略 */
  }
  loadTags();
  drawerOpen.value = true;
}

// ----- 弹框编辑器：新建草稿 + 图片走附件（参考 memos 粘贴/拖拽上传） -----
const EDITOR_DRAFT_KEY = "bm-editor-draft";
const pendingUploads = ref<File[]>([]);

watch(
  () => [form.value.title, form.value.content],
  () => {
    if (editing.value || !drawerOpen.value) return;
    if (form.value.title.trim() || form.value.content.trim()) {
      localStorage.setItem(
        EDITOR_DRAFT_KEY,
        JSON.stringify({ t: form.value.title, c: form.value.content }),
      );
    }
  },
);

async function onEditorUploadImg(files: File[], _callback: (urls: string[]) => void) {
  if (editing.value) {
    // 编辑已有备忘录：直接上传为附件
    try {
      for (const f of files) {
        const bytes = new Uint8Array(await f.arrayBuffer());
        await api.attachmentUpload(editing.value.short_id, f.name, f.type || "", bytes);
      }
      message.success(`已存为附件（${files.length} 个）`);
      await loadEditAttaches(editing.value.short_id);
      loadAttachMap();
    } catch {
      message.error("图片上传失败");
    }
  } else {
    // 新建中：先暂存，保存后统一上传
    pendingUploads.value.push(...files);
    message.info(`图片将在保存后上传为附件（共 ${pendingUploads.value.length} 个）`);
  }
  // 不回调 callback：图片不插入正文，统一走附件区展示
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
      localStorage.removeItem(EDITOR_DRAFT_KEY);
      // 冲销暂存图片 → 附件
      if (pendingUploads.value.length) {
        const files = pendingUploads.value;
        pendingUploads.value = [];
        let ok = 0;
        for (const f of files) {
          try {
            const bytes = new Uint8Array(await f.arrayBuffer());
            await api.attachmentUpload(created.short_id, f.name, f.type || "", bytes);
            ok++;
          } catch {
            /* 单个失败继续 */
          }
        }
        if (ok) message.success(`已上传 ${ok} 张图片附件`);
        loadAttachMap();
      }
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

        <div class="view-toggle">
          <button class="vt-btn" :class="{ on: viewMode === 'timeline' }" title="时间轴" @click="setView('timeline')">
            <FieldTimeOutlined />
          </button>
          <button class="vt-btn" :class="{ on: viewMode === 'card' }" title="卡片网格" @click="setView('card')">
            <AppstoreOutlined />
          </button>
        </div>

        <div class="head-actions">
          <button class="icon-action" title="刷新" @click="load"><ReloadOutlined /></button>
          <button class="icon-action primary" title="完整新建（分类/标签/附件）" @click="openAdd"><PlusOutlined /></button>
        </div>
      </header>

      <div class="panel-layout">
        <!-- 左栏：日历 + 标签 + 统计 -->
        <aside class="side">
          <MemoCalendar :counts="dailyCounts" :selected="filterDate" @select="filterDate = $event" />

          <div class="side-section">
            <div class="side-title">标签</div>
            <div class="tag-cloud">
              <button
                v-for="t in allTags.slice(0, 40)"
                :key="t.tag"
                class="tag-chip"
                :class="{ on: filterTag === t.tag }"
                :title="`按 #${t.tag} 筛选（${t.count} 条）`"
                @click="toggleTagFilter(t.tag)"
              >
                <span class="tag-name">#{{ t.tag }}</span>
                <span class="tag-count">{{ t.count }}</span>
              </button>
              <span v-if="!allTags.length" class="side-empty">暂无标签</span>
            </div>
          </div>

          <div class="side-section side-stats">
            <div class="stat-line"><span>今日</span><b>{{ todayCount }}</b></div>
            <div class="stat-line"><span>本月</span><b>{{ monthCount }}</b></div>
            <div class="stat-line"><span>累计</span><b>{{ memos.length }}</b></div>
          </div>
        </aside>

        <!-- 主区：固定快速输入 + 筛选 chips + memo 流 -->
        <div class="main-col">
          <div class="composer" :class="{ focused }">
            <div class="composer-row">
              <span class="composer-mark"><EditOutlined /></span>
              <textarea
                ref="quickEl"
                v-model="quickText"
                rows="1"
                placeholder="记点什么…（Markdown · Ctrl+Enter 直接记下）"
                @focus="focused = true"
                @blur="focused = false"
                @input="autoGrow"
                @keydown="onQuickKey"
              />
            </div>
            <div class="composer-toolbar">
              <div class="composer-format" title="Markdown 快捷">
                <button class="fmt-btn" title="粗体 **" @click="insertMd('**', '**')"><BoldOutlined /></button>
                <button class="fmt-btn" title="斜体 *" @click="insertMd('*', '*')"><ItalicOutlined /></button>
                <button class="fmt-btn" title="行内代码 `" @click="insertMd('`', '`')"><CodeOutlined /></button>
                <button class="fmt-btn" title="链接 []()" @click="insertMd('[', '](https://)')"><LinkOutlined /></button>
                <button class="fmt-btn" title="列表 - " @click="insertMd('- ')"><UnorderedListOutlined /></button>
              </div>
              <span v-if="hasQuickDraft" class="draft-chip" title="自动保存的草稿">
                草稿已恢复
                <button class="draft-x" title="清空草稿" @click="discardQuickDraft">×</button>
              </span>
              <span v-if="quickText.length" class="composer-count">{{ quickText.length }} 字</span>
              <span class="composer-spacer" />

              <a-dropdown :trigger="['click']">
                <button class="cat-chip" :class="{ set: !!quickCat }" :title="quickCat ? `分类：${quickCat}` : '设置分类'">
                  <template v-if="quickCat">
                    <span
                      class="cat-dot"
                      :style="{ background: catColorMap.get(quickCat) || 'var(--color-primary)' }"
                    />
                    {{ quickCat }}
                    <span class="cat-clear" title="清除分类" @click.stop="quickCat = ''"><CloseOutlined /></span>
                  </template>
                  <template v-else>
                    <FolderOutlined />
                    <span>分类</span>
                  </template>
                </button>
                <template #overlay>
                  <a-menu @click="({ key }: any) => (quickCat = key as string)">
                    <a-menu-item v-for="c in categories" :key="c.name">
                      <span class="cat-menu-dot" :style="{ background: catColorMap.get(c.name) || 'var(--text-tertiary)' }" />
                      {{ c.name }}
                      <span class="cat-menu-count">{{ c.memo_count }}</span>
                    </a-menu-item>
                  </a-menu>
                </template>
              </a-dropdown>

              <a-tooltip title="完整编辑（标题 / 标签 / 附件 / 可见性）">
                <button class="composer-icon" @click="expandQuick"><ExpandOutlined /></button>
              </a-tooltip>
              <button class="composer-send" :disabled="!quickText.trim() || savingQuick" @click="submitQuick">
                <SendOutlined /> {{ savingQuick ? "记下…" : "记下" }}
              </button>
            </div>
          </div>

          <div v-if="filterDate || filterTag || filterCat || searchQuery.trim()" class="chips-row">
            <a-tag v-if="filterDate" closable @close="filterDate = null"><CalendarOutlined /> {{ filterDate }}</a-tag>
            <a-tag v-if="filterTag" closable @close="filterTag = ''">#{{ filterTag }}</a-tag>
            <a-tag v-if="filterCat" closable @close="filterCat = ''">{{ filterCat }}</a-tag>
            <a-tag v-if="searchQuery.trim()" closable @close="searchQuery = ''">「{{ searchQuery.trim() }}」</a-tag>
            <a-tag class="chip-clear" @click="clearFilters">清空筛选</a-tag>
          </div>

          <div class="flow">
            <div v-if="loading" class="memo-empty">加载中…</div>
            <div v-else-if="!memos.length" class="memo-empty hero-empty">
              <EditOutlined class="empty-icon" />
              <p class="empty-title">还没有备忘录</p>
              <p class="empty-sub">在上面记下第一笔 · Ctrl+Enter 秒记</p>
            </div>
            <div v-else-if="!filtered.length" class="memo-empty">无匹配备忘录 — 试试清空筛选</div>

            <!-- 时间轴流（默认） -->
            <MemoTimeline
              v-else-if="viewMode === 'timeline'"
              :memos="filtered"
              :cat-color-map="catColorMap"
              @open="openEdit"
              @pin="togglePin"
              @del="remove"
              @ai="(m: Memo, act: string, lang: string) => openAi(m, act, lang)"
            />

            <!-- 卡片网格 -->
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
                  <a-dropdown :trigger="['click']">
                    <button class="op-btn" title="AI 动作" @click.stop>
                      <svg class="op-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z" />
                        <path d="M20 3v4" /><path d="M22 5h-4" /><path d="M4 17v2" /><path d="M5 18H3" />
                      </svg>
                    </button>
                    <template #overlay>
                      <a-menu @click="({ key }: any) => { const [act, l] = String(key).split(':'); openAi(m, act, l ?? ''); }">
                        <a-menu-item v-for="a in AI_ACTIONS" :key="a.action">{{ a.label }}</a-menu-item>
                        <a-sub-menu title="翻译为">
                          <a-menu-item v-for="l in TRANSLATE_LANGS" :key="`translate:${l}`">{{ l }}</a-menu-item>
                        </a-sub-menu>
                      </a-menu>
                    </template>
                  </a-dropdown>
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

    <!-- 新增/编辑弹框 -->
    <a-modal
      v-model:open="drawerOpen"
      :title="editing ? '编辑备忘录' : '新增备忘录'"
      :width="860"
      centered
      :footer="null"
      destroy-on-close
      wrap-class-name="memo-editor-modal"
    >
      <div class="memo-form">
        <input
          ref="titleInput"
          v-model="form.title"
          class="form-title-input"
          placeholder="标题…（可留空，保存时自动取正文首行）"
          maxlength="200"
        />

        <div class="form-row editor-row">
          <MdEditor
            v-model="form.content"
            :theme="themeStore.isDark ? 'dark' : 'light'"
            language="zh-CN"
            style="height: 400px"
            placeholder="正文（Markdown）…　Ctrl+S 保存 · 粘贴图片自动存为附件"
            :footers="['markdownTotal']"
            :toolbars="['bold', 'underline', 'italic', '-', 'title', 'strikeThrough', 'quote', 'unorderedList', 'orderedList', 'task', '-', 'codeRow', 'code', 'link', 'image', 'table', '-', 'revoke', 'next', 'save', '=', 'pageFullscreen', 'fullscreen', 'preview', 'catalog']"
            @save="save"
            @on-upload-img="onEditorUploadImg"
          />
        </div>

        <div class="meta-row">
          <a-auto-complete
            v-model:value="form.category"
            class="meta-ctl cat"
            placeholder="分类（可新建）"
            allow-clear
            :options="categories.map((c) => ({ value: c.name }))"
          />
          <a-select
            v-model:value="form.tags"
            mode="tags"
            class="meta-ctl tags"
            placeholder="标签（回车确认，支持 a/b 层级）"
            :options="allTags.map((t) => ({ value: t.tag }))"
          />
          <span class="meta-divider" />
          <a-select v-model:value="form.visibility" class="meta-ctl vis" :options="VISIBILITY_OPTIONS" />
          <a-tooltip :title="form.pinned ? '已置顶' : '置顶'">
            <button class="pin-toggle" :class="{ on: form.pinned }" @click="form.pinned = !form.pinned">
              <PushpinFilled v-if="form.pinned" /><PushpinOutlined v-else />
            </button>
          </a-tooltip>
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
        <div v-else class="attach-hint">
          <PaperClipOutlined class="attach-hint-icon" />
          保存后可添加附件 · 编辑器里粘贴/选择图片会在保存时自动上传为附件
        </div>

        <div class="form-actions">
          <span class="form-hint">每次保存自动记录版本，可在版本历史中回滚</span>
          <span class="composer-spacer" />
          <button class="btn-cancel" @click="drawerOpen = false">取消</button>
          <button class="btn-save" :disabled="saving" @click="save">
            {{ saving ? "保存中…" : editing ? "保存" : "添加" }}
          </button>
        </div>
      </div>
    </a-modal>
    <!-- AI 结果弹框（流式） -->
    <AiResultModal
      v-model:open="aiModal.open"
      :memo="aiModal.memo"
      :action="aiModal.action"
      @apply="onAiApply"
    />
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
  font-family: "Noto Serif SC", "Source Han Serif SC", "STZhongsong", serif;
  font-size: 15.5px;
  font-weight: 600;
  letter-spacing: 0.04em;
  color: var(--text-primary);
}

.head-count {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
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

/* ===== 三区布局：左栏（日历/标签/统计）+ 主区（输入/流） ===== */
.panel-layout {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 300px 1fr;
}

.side {
  min-height: 0;
  overflow: auto;
  padding: 12px;
  border-right: 1px solid var(--border-light);
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.side-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.side-title {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.12em;
  color: var(--text-tertiary);
}
.tag-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.tag-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2.5px 10px;
  font-size: 11.5px;
  color: var(--text-secondary);
  background: transparent;
  border: none;
  border-radius: 999px;
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}
.tag-chip:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 5%, transparent);
}
.tag-chip.on {
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  font-weight: 600;
}
.tag-count {
  font-size: 10px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
}
.tag-chip.on .tag-count {
  color: color-mix(in srgb, var(--hero-cta-text, #fff) 75%, transparent);
}
.side-empty {
  font-size: 11.5px;
  color: var(--text-tertiary);
}

.side-stats {
  margin-top: auto;
  border-top: 1px dashed var(--border-light);
  padding-top: 10px;
  gap: 6px;
}
.stat-line {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-secondary);
}
.stat-line b {
  font-family: "JetBrains Mono", monospace;
  font-size: 13px;
  color: var(--text-primary);
}

.main-col {
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px 14px 8px;
}

/* ===== 顶部固定快速输入（与流区同轴的固定列宽） ===== */
.composer {
  flex-shrink: 0;
  width: 100%;
  max-width: 780px;
  margin: 0 auto;
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-lg, 14px);
  padding: 12px 14px 8px;
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast), background var(--transition-fast);
}
.composer.focused {
  border-color: color-mix(in srgb, var(--color-primary) 55%, var(--border-light));
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 10%, transparent);
  background: var(--bg-surface);
}

.composer-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}
.composer-mark {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  flex-shrink: 0;
  margin-top: 2px;
  font-size: 14px;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  border-radius: 10px;
}
.composer.focused .composer-mark {
  background: color-mix(in srgb, var(--color-primary) 16%, transparent);
}
.composer textarea {
  display: block;
  flex: 1;
  width: 100%;
  min-height: 34px;
  max-height: 150px;
  padding: 6px 0 2px;
  resize: none;
  border: none;
  outline: none;
  background: transparent;
  font-size: 13.5px;
  line-height: 1.65;
  color: var(--text-primary);
  font-family: inherit;
}
.composer textarea::placeholder {
  color: var(--text-tertiary);
}

.composer-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
  padding-top: 6px;
  border-top: 1px solid color-mix(in srgb, var(--border-light) 70%, transparent);
}
.composer-format {
  display: flex;
  align-items: center;
  gap: 1px;
  padding-right: 6px;
  margin-right: 2px;
  border-right: 1px solid color-mix(in srgb, var(--border-light) 70%, transparent);
}
.fmt-btn {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  font-size: 12px;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  transition: color var(--transition-fast), background var(--transition-fast);
}
.fmt-btn:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
}
.draft-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 10.5px;
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  border-radius: 999px;
  padding: 1.5px 4px 1.5px 8px;
  white-space: nowrap;
}
.draft-x {
  display: grid;
  place-items: center;
  width: 14px;
  height: 14px;
  font-size: 11px;
  line-height: 1;
  color: inherit;
  background: transparent;
  border: none;
  border-radius: 50%;
  cursor: pointer;
}
.draft-x:hover {
  background: color-mix(in srgb, var(--color-primary) 20%, transparent);
}
/* 分类紧凑胶囊（贴右侧，宽度自适应内容） */
.cat-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  max-width: 180px;
  height: 24px;
  padding: 0 9px;
  font-size: 11.5px;
  color: var(--text-tertiary);
  background: transparent;
  border: 1px solid var(--border-light);
  border-radius: 999px;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: all var(--transition-fast);
}
.cat-chip:hover {
  color: var(--text-primary);
  border-color: var(--text-tertiary);
}
.cat-chip.set {
  color: var(--text-primary);
  border-color: color-mix(in srgb, var(--color-primary) 40%, var(--border-light));
  background: color-mix(in srgb, var(--color-primary) 6%, transparent);
}
.cat-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}
.cat-clear {
  display: grid;
  place-items: center;
  width: 14px;
  height: 14px;
  margin-right: -3px;
  font-size: 9px;
  border-radius: 50%;
  color: var(--text-tertiary);
}
.cat-clear:hover {
  color: #ef4444;
  background: color-mix(in srgb, #ef4444 10%, transparent);
}
.cat-menu-dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  margin-right: 8px;
  vertical-align: middle;
}
.cat-menu-count {
  float: right;
  margin-left: 16px;
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
}
.composer-count {
  font-size: 10.5px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
}
.composer-spacer {
  flex: 1;
}
.composer-icon {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  font-size: 13px;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: 999px;
  cursor: pointer;
  transition: color var(--transition-fast), background var(--transition-fast);
}
.composer-icon:hover {
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
}
.composer-send {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 28px;
  padding: 0 14px;
  font-size: 12px;
  font-weight: 600;
  color: var(--hero-cta-text, #fff);
  background: linear-gradient(
    135deg,
    var(--color-primary),
    color-mix(in srgb, var(--color-primary) 82%, #000)
  );
  border: none;
  border-radius: 999px;
  cursor: pointer;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--color-primary) 26%, transparent);
  transition: filter var(--transition-fast), box-shadow var(--transition-fast);
}
.composer-send:hover {
  filter: brightness(1.08);
  box-shadow: 0 3px 10px color-mix(in srgb, var(--color-primary) 34%, transparent);
}
.composer-send:disabled {
  opacity: 0.45;
  cursor: default;
  box-shadow: none;
}

/* ===== 筛选 chips ===== */
.chips-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  flex-shrink: 0;
  width: 100%;
  max-width: 780px;
  margin: 0 auto;
}
.chips-row :deep(.ant-tag) {
  margin-right: 0;
  font-size: 11.5px;
  line-height: 20px;
  padding: 0 8px;
  border-radius: 999px;
}
.chip-clear {
  cursor: pointer;
  color: var(--color-primary);
  border-color: var(--color-primary-border);
}

/* ===== 流区（固定 780px 阅读列，居中） ===== */
.flow {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 2px 4px 8px 2px;
}
.flow > :deep(*),
.flow > * {
  width: 100%;
  max-width: 780px;
  margin-left: auto;
  margin-right: auto;
}

.memo-empty {
  display: grid;
  place-items: center;
  height: 200px;
  color: var(--text-tertiary);
  font-size: 13px;
}
.hero-empty {
  height: auto;
  padding: 70px 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  text-align: center;
}
.empty-icon {
  font-size: 34px;
  opacity: 0.55;
}
.empty-title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
}
.empty-sub {
  margin: 0;
  font-size: 12px;
}

.memo-pin-mark {
  font-size: 12px;
  color: var(--color-primary);
  flex-shrink: 0;
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
/* 卡片视图操作行：悬浮时浮在卡片底部（毛玻璃胶囊，缓入不跳动） */
.memo-card-ops {
  position: absolute;
  right: 8px;
  bottom: 8px;
  background: color-mix(in srgb, var(--bg-surface) 86%, transparent);
  backdrop-filter: blur(8px);
  border: 1px solid var(--border-light);
  border-radius: 999px;
  padding: 2px;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--text-primary) 8%, transparent);
  transform: translateY(3px);
  transition: opacity var(--transition-fast), transform var(--transition-fast);
}
.memo-card:hover .memo-card-ops {
  transform: translateY(0);
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
  border-radius: 999px;
  cursor: pointer;
  transition: color var(--transition-fast), background var(--transition-fast);
}
.op-btn:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 7%, transparent);
}
.op-btn.on {
  color: var(--color-primary);
}
.op-btn.danger:hover {
  color: #ef4444;
}
.op-btn .op-svg {
  width: 14px;
  height: 14px;
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
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
  min-height: 120px;
}
.memo-card:hover {
  border-color: color-mix(in srgb, var(--color-primary) 38%, var(--border-light));
  box-shadow: 0 2px 12px color-mix(in srgb, var(--text-primary) 8%, transparent);
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

/* ===== 编辑弹框表单 ===== */
.memo-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.form-row {
  display: flex;
  gap: 10px;
}

/* 标题：无边框大字（Notion 式首行） */
.form-title-input {
  width: 100%;
  padding: 2px 2px 8px;
  font-size: 17px;
  font-weight: 600;
  letter-spacing: -0.2px;
  color: var(--text-primary);
  background: transparent;
  border: none;
  border-bottom: 1px solid color-mix(in srgb, var(--border-light) 60%, transparent);
  border-radius: 0;
  outline: none;
  transition: border-color var(--transition-fast);
}
.form-title-input:focus {
  border-bottom-color: var(--color-primary);
}
.form-title-input::placeholder {
  color: var(--text-tertiary);
  font-weight: 500;
}

/* 编辑器：融入主题（透明底 + 主题色映射，修复暗色下黑块） */
.editor-row {
  flex: none;
}
.editor-row :deep(.md-editor) {
  --md-bk-color: var(--bg-input);
  --md-color: var(--text-primary);
  --md-border-color: var(--border-light);
  --md-hover-color: var(--text-secondary);
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  overflow: hidden;
}
.editor-row :deep(.md-editor .cm-editor),
.editor-row :deep(.md-editor .cm-scroller),
.editor-row :deep(.md-editor .cm-gutters),
.editor-row :deep(.md-editor-toolbar-wrapper),
.editor-row :deep(.md-editor-toolbar),
.editor-row :deep(.md-editor-footer),
.editor-row :deep(.md-editor-previewWrapper),
.editor-row :deep(.md-editor-preview) {
  background: transparent;
}
.editor-row :deep(.md-editor .cm-gutters) {
  border-right: 1px solid color-mix(in srgb, var(--border-light) 60%, transparent);
}
.editor-row :deep(.md-editor-toolbar-item) {
  color: var(--text-secondary);
}
/* 工具栏紧凑化：窄弹框内完整显示，超出可横向滚动兜底 */
.editor-row :deep(.md-editor-toolbar-wrapper) {
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: thin;
}
.editor-row :deep(.md-editor-toolbar) {
  flex-wrap: nowrap;
}
.editor-row :deep(.md-editor-toolbar-item) {
  width: 26px;
  height: 26px;
  margin: 0 1px;
}
.editor-row :deep(.md-editor-toolbar-divider) {
  margin: 0 4px;
}

/* 元信息条：分类 / 标签 / 可见性 / 置顶——安静芯片 */
.meta-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid color-mix(in srgb, var(--border-light) 60%, transparent);
}
.meta-ctl {
  background: var(--bg-input);
  border-radius: 8px;
}
.meta-ctl.cat {
  width: 170px;
}
.meta-ctl.tags {
  flex: 1;
  min-width: 160px;
}
.meta-ctl.vis {
  width: 110px;
}
.meta-row :deep(.ant-select .ant-select-selector),
.meta-row :deep(.ant-select-single:not(.ant-select-customize-input) .ant-select-selector) {
  border: none !important;
  background: transparent !important;
  box-shadow: none !important;
  font-size: 12.5px;
  padding-left: 10px;
}
.meta-row :deep(.ant-select-single .ant-select-selector) {
  height: 30px !important;
}
.meta-row :deep(.ant-select-single .ant-select-selection-item),
.meta-row :deep(.ant-select-single .ant-select-selection-search) {
  line-height: 28px !important;
}
.meta-row :deep(.ant-select:not(.ant-select-open):hover) {
  background: color-mix(in srgb, var(--text-primary) 4%, var(--bg-input));
}
.meta-divider {
  width: 1px;
  height: 18px;
  background: var(--border-light);
  flex-shrink: 0;
}
.pin-toggle {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  flex-shrink: 0;
  font-size: 14px;
  color: var(--text-tertiary);
  background: var(--bg-input);
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.pin-toggle:hover {
  color: var(--text-primary);
}
.pin-toggle.on {
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 12%, transparent);
}

.form-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 12px;
}
.form-hint {
  font-size: 11px;
  color: var(--text-tertiary);
}
.btn-cancel {
  height: 30px;
  padding: 0 16px;
  font-size: 12.5px;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid var(--border-light);
  border-radius: 999px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-cancel:hover {
  color: var(--text-primary);
  border-color: var(--text-tertiary);
}
.btn-save {
  height: 30px;
  padding: 0 20px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--hero-cta-text, #fff);
  background: linear-gradient(
    135deg,
    var(--color-primary),
    color-mix(in srgb, var(--color-primary) 82%, #000)
  );
  border: none;
  border-radius: 999px;
  cursor: pointer;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--color-primary) 26%, transparent);
  transition: filter var(--transition-fast), box-shadow var(--transition-fast);
}
.btn-save:hover {
  filter: brightness(1.08);
}
.btn-save:disabled {
  opacity: 0.5;
  cursor: default;
  box-shadow: none;
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
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11.5px;
  color: var(--text-tertiary);
  padding: 6px 2px;
}
.attach-hint-icon {
  font-size: 12px;
  color: var(--text-tertiary);
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

<style>
/* 编辑弹框：融入主题的容器（Modal teleport 到 body，须用全局样式） */
.memo-editor-modal .ant-modal-content {
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: 16px;
  padding: 0;
  overflow: hidden;
}
.memo-editor-modal .ant-modal-header {
  padding: 16px 22px 0;
  background: transparent;
  border: none;
}
.memo-editor-modal .ant-modal-title {
  font-family: "Noto Serif SC", "Source Han Serif SC", "STZhongsong", serif;
  font-size: 15.5px;
  font-weight: 600;
  letter-spacing: 0.04em;
  color: var(--text-primary);
}
.memo-editor-modal .ant-modal-body {
  padding: 12px 22px 18px;
  max-height: 76vh;
  overflow: auto;
}
.memo-editor-modal .ant-modal-close {
  top: 12px;
  right: 16px;
  color: var(--text-tertiary);
}
.memo-editor-modal .ant-modal-close:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
}
/* 弹框内的下拉/自动补全面板同样走主题色 */
.memo-editor-modal .ant-select-dropdown {
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
}
</style>
