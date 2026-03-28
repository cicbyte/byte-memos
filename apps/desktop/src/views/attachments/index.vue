<script setup lang="ts">
/**
 * 附件库 /attachments（byte-memos 附件管理域）
 *
 * 参考 usememos/memos /attachments 资源页（wiki: attachment.md §资源库）：
 * 媒体网格 / 音频 / 文档 / 未关联 四个 tab + 游离附件批量清理。
 * 图片经 ObjectURL 缓存预览（a-image 支持放大），非媒体走行列表。
 */
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import {
  ReloadOutlined,
  PaperClipOutlined,
  DownloadOutlined,
  DeleteOutlined,
  FileImageOutlined,
  FileTextOutlined,
  SoundOutlined,
  LinkOutlined,
} from "@ant-design/icons-vue";
import { message, Modal } from "ant-design-vue";
import { api } from "@/api";
import type { AttachmentOverview } from "@/api/types";
import { attachmentObjectUrl, revokeAttachmentUrl, fmtSize } from "@/api/attachments";
import { useLayoutStore } from "@/stores/layout";
import EmptyState from "@/components/common/EmptyState.vue";

const router = useRouter();
const layout = useLayoutStore();

const items = ref<AttachmentOverview[]>([]);
const urls = ref<Record<number, string>>({});
const loading = ref(false);
const tab = ref<"media" | "audio" | "docs" | "orphan">("media");

const isMedia = (m: string) => m.startsWith("image/") || m.startsWith("video/");
const isAudio = (m: string) => m.startsWith("audio/");

const filtered = computed(() => {
  if (tab.value === "media") return items.value.filter((a) => isMedia(a.mime_type));
  if (tab.value === "audio") return items.value.filter((a) => isAudio(a.mime_type));
  if (tab.value === "docs") return items.value.filter((a) => !isMedia(a.mime_type) && !isAudio(a.mime_type));
  return items.value.filter((a) => a.orphan);
});

const orphanCount = computed(() => items.value.filter((a) => a.orphan).length);

async function load() {
  loading.value = true;
  try {
    items.value = await api.attachmentListAll();
    // 预取媒体缩略 ObjectURL
    const next: Record<number, string> = {};
    for (const a of items.value) {
      if (a.mime_type.startsWith("image/")) {
        try {
          next[a.id] = await attachmentObjectUrl(a.id);
        } catch {
          /* 忽略单个失败 */
        }
      }
    }
    // 释放不再存在的缓存
    for (const id of Object.keys(urls.value).map(Number)) {
      if (!next[id]) revokeAttachmentUrl(id);
    }
    urls.value = next;
  } catch (e: any) {
    message.error(typeof e === "string" ? e : "加载失败");
  } finally {
    loading.value = false;
  }
  updateFooter();
}

function updateFooter() {
  layout.setStatusItems([
    { key: "view", text: "附件库" },
    { key: "count", text: `共 ${items.value.length} 个` + (orphanCount.value ? ` · 未关联 ${orphanCount.value}` : "") },
  ]);
}

function openMemo(a: AttachmentOverview) {
  if (a.orphan) return;
  router.push({ path: "/memos", query: { id: a.memo_short_id } });
}

async function download(a: AttachmentOverview) {
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

function remove(a: AttachmentOverview) {
  Modal.confirm({
    title: "删除附件",
    content: `确认删除「${a.filename}」？${a.storage_type === "FILE" ? "磁盘文件将一并删除。" : ""}`,
    okType: "danger",
    onOk: async () => {
      try {
        await api.attachmentDelete(a.id);
        revokeAttachmentUrl(a.id);
        message.success("已删除");
        await load();
      } catch {
        message.error("删除失败");
      }
    },
  });
}

function cleanOrphans() {
  const orphans = items.value.filter((a) => a.orphan);
  if (!orphans.length) return;
  Modal.confirm({
    title: "清理未关联附件",
    content: `共 ${orphans.length} 个附件所属备忘录已被彻底删除。将删除这些附件记录与文件，不可恢复。`,
    okType: "danger",
    okText: "清理",
    onOk: async () => {
      let ok = 0;
      for (const a of orphans) {
        try {
          await api.attachmentDelete(a.id);
          revokeAttachmentUrl(a.id);
          ok++;
        } catch {
          /* 单个失败继续 */
        }
      }
      message.success(`已清理 ${ok} 个未关联附件`);
      await load();
    },
  });
}

function typeIcon(a: AttachmentOverview) {
  if (isMedia(a.mime_type)) return FileImageOutlined;
  if (isAudio(a.mime_type)) return SoundOutlined;
  return FileTextOutlined;
}

/** 音频行内取 ObjectURL（懒加载，点播放时才拉字节） */
async function playAudio(a: AttachmentOverview) {
  if (!urls.value[a.id]) {
    try {
      urls.value = { ...urls.value, [a.id]: await attachmentObjectUrl(a.id) };
    } catch {
      message.error("读取音频失败");
      return;
    }
  }
}

onMounted(load);
</script>

<template>
  <div class="attach-view">
    <section class="panel">
      <header class="panel-head">
        <span class="head-title">附件库</span>
        <span class="head-count">{{ items.length }}</span>

        <a-radio-group v-model:value="tab" size="small" class="attach-tabs">
          <a-radio-button value="media">媒体</a-radio-button>
          <a-radio-button value="audio">音频</a-radio-button>
          <a-radio-button value="docs">文档</a-radio-button>
          <a-radio-button value="orphan">
            未关联<template v-if="orphanCount">（{{ orphanCount }}）</template>
          </a-radio-button>
        </a-radio-group>

        <div class="head-actions">
          <button
            v-if="tab === 'orphan' && orphanCount"
            class="icon-action danger"
            title="清理全部未关联附件"
            @click="cleanOrphans"
          >
            <DeleteOutlined /> 清理
          </button>
          <button class="icon-action" title="刷新" @click="load"><ReloadOutlined /></button>
        </div>
      </header>

      <div class="panel-body">
        <div v-if="loading" class="empty-tip">加载中…</div>
        <EmptyState
          v-else-if="!filtered.length"
          name="clip"
          :text="tab === 'orphan' ? '没有未关联的附件' : '暂无此类附件'"
          :desc="tab === 'orphan' ? '所有附件都已关联到备忘录' : '在备忘录编辑抽屉里「添加附件」上传'"
          inline
        />

        <!-- 媒体网格 -->
        <template v-else-if="tab === 'media'">
          <a-image-preview-group>
            <div class="media-grid">
              <div v-for="a in filtered" :key="a.id" class="media-card">
                <a-image
                  v-if="urls[a.id]"
                  :src="urls[a.id]"
                  :alt="a.filename"
                  :width="'100%'"
                  :height="110"
                  style="object-fit: cover; border-radius: var(--radius-xs)"
                  class="media-thumb"
                />
                <div v-else class="media-fallback"><PaperClipOutlined /></div>
                <div class="media-meta">
                  <span class="media-name" :title="a.filename">{{ a.filename }}</span>
                  <span class="media-size">{{ fmtSize(a.file_size) }}</span>
                </div>
                <div class="media-ops">
                  <button v-if="!a.orphan" class="op-btn" title="打开所属笔记" @click="openMemo(a)"><LinkOutlined /></button>
                  <button class="op-btn" title="下载" @click="download(a)"><DownloadOutlined /></button>
                  <button class="op-btn danger" title="删除" @click="remove(a)"><DeleteOutlined /></button>
                </div>
              </div>
            </div>
          </a-image-preview-group>
        </template>

        <!-- 音频/文档行列表 -->
        <template v-else>
          <div class="row-list">
            <div v-for="a in filtered" :key="a.id" class="row-item" :class="{ orphan: a.orphan }">
              <span class="row-icon"><component :is="typeIcon(a)" /></span>
              <div class="row-main">
                <span class="row-name" :title="a.filename">{{ a.filename }}</span>
                <span class="row-sub">
                  {{ fmtSize(a.file_size) }} · {{ a.created_at.replace("T", " ").slice(0, 16) }}
                  <template v-if="a.orphan"> · 所属笔记已删除</template>
                </span>
              </div>
              <div class="row-ops">
                <button v-if="!a.orphan" class="op-btn" title="打开所属笔记" @click="openMemo(a)"><LinkOutlined /></button>
                <button v-if="isAudio(a.mime_type)" class="op-btn" title="播放" @click="playAudio(a)">
                  <SoundOutlined />
                </button>
                <button class="op-btn" title="下载" @click="download(a)"><DownloadOutlined /></button>
                <button class="op-btn danger" title="删除" @click="remove(a)"><DeleteOutlined /></button>
              </div>
            </div>
          </div>
        </template>
      </div>
    </section>
  </div>
</template>

<style scoped>
.attach-view {
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
.attach-tabs {
  margin-left: 6px;
}
.head-actions {
  display: flex;
  gap: 6px;
  margin-left: auto;
}
.icon-action {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 28px;
  padding: 0 8px;
  font-size: 13px;
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
.icon-action.danger {
  color: #ef4444;
  border-color: #fca5a5;
}
.icon-action.danger:hover {
  background: #fef2f2;
}
.panel-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 10px;
}
.empty-tip {
  display: grid;
  place-items: center;
  height: 200px;
  color: var(--text-tertiary);
  font-size: 13px;
}

/* 媒体网格 */
.media-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 10px;
}
.media-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px;
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}
.media-card:hover {
  border-color: var(--color-primary-border);
  transform: translateY(-1px);
}
.media-thumb {
  width: 100%;
  height: 110px;
  object-fit: cover;
  border-radius: var(--radius-xs);
  cursor: zoom-in;
}
.media-fallback {
  display: grid;
  place-items: center;
  height: 110px;
  font-size: 22px;
  color: var(--text-tertiary);
  background: var(--bg-elevated);
  border-radius: var(--radius-xs);
}
.media-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}
.media-name {
  flex: 1;
  min-width: 0;
  font-size: 11.5px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.media-size {
  flex-shrink: 0;
  font-size: 10.5px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
}
.media-ops,
.row-ops {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity var(--transition-fast);
}
.media-card:hover .media-ops,
.row-item:hover .row-ops {
  opacity: 1;
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
  background: var(--bg-surface);
}
.op-btn.danger:hover {
  color: #ef4444;
}

/* 行列表 */
.row-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-width: 880px;
}
.row-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  transition: background var(--transition-fast);
}
.row-item:hover {
  background: var(--color-hover);
}
.row-item.orphan {
  opacity: 0.75;
  border-style: dashed;
}
.row-icon {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  font-size: 15px;
  color: var(--text-tertiary);
  background: var(--bg-elevated);
  border-radius: var(--radius-xs);
  flex-shrink: 0;
}
.row-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.row-name {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.row-sub {
  font-size: 11px;
  color: var(--text-tertiary);
}
</style>
