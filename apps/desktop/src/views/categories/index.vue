<script setup lang="ts">
/**
 * 分类页 /categories —— 分类维护 + 概览
 *
 * 左：分类列表（新建/行内重命名/删除），双徽标显示项目数+片段数
 * 右：选中分类的概览卡（项目数/片段数/笔记数，点击卡片跳转对应页面带分类筛选，数量为 0 亦可跳转）
 *
 * 结构：
 * - 状态 + 逻辑：./lib/useCategories
 * - 左列表：./components/CategoryList
 * - 右概览：./components/CategoryDetail
 * 本文件仅保留生命周期与布局。
 */
import { onMounted, onUnmounted } from "vue";
import CategoryList from "./components/CategoryList.vue";
import CategoryDetail from "./components/CategoryDetail.vue";
import { useCategories } from "./lib/useCategories";

const {
  categories,
  selectedId,
  loading,
  previewBookmarks,
  detailTab,
  newName,
  creating,
  editingId,
  editingName,
  selected,
  tabs,
  load,
  selectCat,
  startCreate,
  cancelCreate,
  onCreate,
  startEdit,
  commitEdit,
  cancelEdit,
  onRemove,
  goBookmarks,
  handleEsc,
} = useCategories();

onMounted(() => {
  load();
  window.addEventListener("keydown", handleEsc);
});
onUnmounted(() => window.removeEventListener("keydown", handleEsc));
</script>

<template>
  <div class="cat-view">
    <!-- 左：分类列表 -->
    <CategoryList
      :categories="categories"
      :loading="loading"
      :selected-id="selectedId"
      :creating="creating"
      :editing-id="editingId"
      v-model:new-name="newName"
      v-model:editing-name="editingName"
      @start-create="startCreate"
      @cancel-create="cancelCreate"
      @create="onCreate"
      @select="selectCat"
      @start-edit="startEdit"
      @commit-edit="commitEdit"
      @cancel-edit="cancelEdit"
      @remove="onRemove"
    />

    <!-- 右：概览卡 -->
    <CategoryDetail
      :categories="categories"
      :selected="selected"
      :tabs="tabs"
      :preview-bookmarks="previewBookmarks"
      v-model:detail-tab="detailTab"
      @start-edit="startEdit"
      @remove="onRemove"
      @go-bookmarks="goBookmarks"
    />
  </div>
</template>

<style scoped>
.cat-view {
  display: grid;
  grid-template-columns: 320px 1fr;
  grid-template-rows: minmax(0, 1fr); /* 行撑满容器高度，保证各列 panel 等高 */
  gap: var(--spacing-md);
  height: 100%;
  padding: var(--spacing-md);
}
</style>
