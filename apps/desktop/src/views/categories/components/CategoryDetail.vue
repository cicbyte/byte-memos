<script setup lang="ts">
/**
 * 分类详情概览（categories → 右 Panel，书签专用）
 *
 * 可变状态（detailTab）通过 defineModel 与主入口双向绑定，
 * 状态唯一来源仍在主入口的 useCategories；本组件不持有同名状态副本。
 * 统计卡跳转/重命名/删除以事件上报，由主入口调用 composable 函数；
 * 预览项点击跳转为纯导航，本组件直接 useRouter。
 */
import { useRouter } from "vue-router";
import {
  EditOutlined,
  DeleteOutlined,
  FolderOutlined,
  LinkOutlined,
} from "@ant-design/icons-vue";
import EmptyState from "@/components/common/EmptyState.vue";
import Panel from "@/components/shared/Panel.vue";
import DetailTabs from "@/components/shared/DetailTabs.vue";
import StatCard from "@/components/shared/StatCard.vue";
import PreviewItem from "@/components/shared/PreviewItem.vue";
import type { Category, Bookmark } from "@/api/types";

defineProps<{
  categories: Category[];
  selected: Category | null;
  tabs: { key: string; label: string; count?: number }[];
  previewBookmarks: Bookmark[];
}>();

const detailTab = defineModel<string>("detailTab", { required: true });

const emit = defineEmits<{
  startEdit: [c: Category];
  remove: [c: Category];
  goBookmarks: [];
}>();

const router = useRouter();
</script>

<template>
  <Panel>
    <template v-if="selected" #head>
      <FolderOutlined class="head-icon" />
      <span class="head-title">{{ selected.name }}</span>
      <div class="head-actions">
        <button class="icon-action" title="重命名" @click="emit('startEdit', selected)">
          <EditOutlined />
        </button>
        <button class="icon-action danger" title="删除" @click="emit('remove', selected)">
          <DeleteOutlined />
        </button>
      </div>
    </template>

    <template v-if="selected">
      <DetailTabs v-model="detailTab" :tabs="tabs" />

      <div class="detail-tab-body">
        <!-- 概览 tab -->
        <div v-if="detailTab === 'overview'" class="overview-body">
          <div class="stat-grid">
            <StatCard
              :icon="LinkOutlined" variant="bookmark"
              :value="selected.bookmark_count" label="书签" go
              @click="emit('goBookmarks')"
            />
          </div>
        </div>

        <!-- 书签 tab -->
        <div v-else-if="detailTab === 'bookmarks'" class="tab-list-body">
          <div v-if="!previewBookmarks.length" class="tab-empty">该分类下暂无书签</div>
          <PreviewItem
            v-for="b in previewBookmarks"
            :key="b.short_id"
            :icon="LinkOutlined" variant="bookmark"
            :title="b.title || b.url" :desc="b.url"
            @click="router.push({ path: '/bookmarks', query: { id: b.short_id } })"
          />
        </div>
      </div>
    </template>

    <template v-if="!selected" #empty>
      <EmptyState
        :icon="FolderOutlined"
        text="从左侧选择分类查看概览"
        :hint="!categories.length ? '或先在左侧创建一个分类' : ''"
      />
    </template>
  </Panel>
</template>

<style scoped>
/* ===== 右概览 ===== */
.overview-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-lg);
}
.stat-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: var(--spacing-md);
}
.detail-tab-body {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}
.tab-list-body {
  padding: var(--spacing-sm);
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.tab-empty {
  padding: 32px 16px;
  text-align: center;
  font-size: 12px;
  color: var(--text-disabled);
}
</style>
