<script setup lang="ts">
/**
 * 标签页 /tags（书签专用）
 *
 * 双栏布局：左侧标签云（字号按使用频率缩放） + 右侧选中标签的关联书签。
 */
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { TagsOutlined, LinkOutlined } from "@ant-design/icons-vue";
import EmptyState from "@/components/common/EmptyState.vue";
import Panel from "@/components/shared/Panel.vue";
import PreviewItem from "@/components/shared/PreviewItem.vue";
import { api } from "@/api";
import type { TagStat, Bookmark } from "@/api/types";
import { useLayoutStore } from "@/stores/layout";

const router = useRouter();
const layout = useLayoutStore();

const tags = ref<TagStat[]>([]);
const selectedTag = ref<string | null>(null);
const loading = ref(false);

/** 选中标签的关联书签 */
const tagBookmarks = ref<Bookmark[]>([]);
const detailLoading = ref(false);

const maxCount = computed(() => Math.max(1, ...tags.value.map((t) => t.count)));

/** 字号按使用频率缩放（12-22px） */
function fontSize(count: number): string {
  const ratio = count / maxCount.value;
  return `${12 + ratio * 10}px`;
}

/** 关联总数 */
const totalCount = computed(() => tagBookmarks.value.length);

async function loadTags() {
  loading.value = true;
  try {
    tags.value = await api.tagStats(200);
    layout.setStatusItems([
      { key: "view", text: "标签" },
      { key: "count", text: `${tags.value.length} 个` },
    ]);
  } finally {
    loading.value = false;
  }
}

async function selectTag(name: string) {
  selectedTag.value = name;
  detailLoading.value = true;
  try {
    tagBookmarks.value = await api.listBookmarks({ tag: name, limit: 100 });
  } finally {
    detailLoading.value = false;
  }
}

onMounted(loadTags);
</script>

<template>
  <div class="tags-view">
    <!-- 左栏：标签云 -->
    <Panel title="标签" :count="tags.length">
      <div class="panel-body">
        <div class="cloud">
          <span
            v-for="t in tags"
            :key="t.tag"
            class="tag-chip"
            :class="{ active: selectedTag === t.tag }"
            :style="{ fontSize: fontSize(t.count) }"
            @click="selectTag(t.tag)"
          >
            #{{ t.tag }}
            <span class="chip-count">{{ t.count }}</span>
          </span>
        </div>
      </div>
      <template #empty>
        <EmptyState
          v-if="!tags.length && !loading"
          :icon="TagsOutlined"
          text="暂无标签"
          hint="在书签页给书签添加标签"
        />
      </template>
    </Panel>

    <!-- 右栏：关联书签 -->
    <Panel>
      <template v-if="selectedTag" #head>
        <span class="head-title">#{{ selectedTag }}</span>
        <span class="head-count">{{ totalCount }}</span>
      </template>

      <div v-if="selectedTag" class="detail-body">
        <div v-if="tagBookmarks.length" class="rel-group">
          <div class="group-head">
            <LinkOutlined class="group-icon bookmark" />
            <span class="group-label">书签</span>
            <span class="group-count">{{ tagBookmarks.length }}</span>
          </div>
          <PreviewItem
            v-for="b in tagBookmarks"
            :key="b.short_id"
            :icon="LinkOutlined"
            variant="bookmark"
            :title="b.title || b.url"
            :desc="b.url"
            @click="router.push(`/bookmarks?id=${b.short_id}`)"
          />
        </div>

        <EmptyState
          v-if="!totalCount && !detailLoading"
          :icon="TagsOutlined"
          text="无关联书签"
          size="small"
          inline
        />
      </div>

      <template v-if="!selectedTag" #empty>
        <EmptyState
          :icon="TagsOutlined"
          text="从左侧选择标签"
          hint="查看该标签关联的书签"
        />
      </template>
    </Panel>
  </div>
</template>

<style scoped>
.tags-view {
  display: grid;
  grid-template-columns: 340px 1fr;
  grid-template-rows: minmax(0, 1fr); /* 行撑满容器高度，保证各列 panel 等高 */
  gap: var(--spacing-md);
  height: 100%;
  padding: var(--spacing-md);
}

/* ============ 标签云 ============ */
.cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 10px;
  padding: var(--spacing-xs);
}
.tag-chip {
  display: inline-flex;
  align-items: baseline;
  gap: 4px;
  padding: 3px 8px;
  border-radius: var(--radius-sm);
  color: var(--color-primary);
  cursor: pointer;
  font-weight: 500;
  transition: background var(--transition-fast);
  user-select: none;
}
.tag-chip:hover {
  background: var(--bg-elevated);
}
.tag-chip.active {
  background: var(--color-primary-bg);
  color: var(--color-primary-active);
}
.chip-count {
  font-size: 10px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
}
.tag-chip.active .chip-count {
  color: var(--color-primary);
}

/* ============ 右栏关联内容 ============ */
.detail-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-md);
}
.rel-group {
  margin-bottom: var(--spacing-lg);
}
.rel-group:last-child {
  margin-bottom: 0;
}
.group-head {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 8px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border-light);
}
.group-icon {
  font-size: 14px;
}
.group-icon.bookmark { color: var(--color-primary); }
.group-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
}
.group-count {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
  background: var(--bg-input);
  padding: 0 6px;
  border-radius: 8px;
}
</style>
