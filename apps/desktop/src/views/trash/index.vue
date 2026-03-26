<script setup lang="ts">
/**
 * 回收站页 /trash（书签专用）
 *
 * 展示已软删除的书签，支持恢复与彻底删除。
 * 逻辑见 useTrash.ts，样式见 trash.css。
 */
import {
  DeleteOutlined,
  LinkOutlined,
  UndoOutlined,
  ReloadOutlined,
} from "@ant-design/icons-vue";
import EmptyState from "@/components/common/EmptyState.vue";
import { useTrash } from "./lib/useTrash";
import "./lib/trash.css";

const {
  items, loading, filtered,
  load, restore, purge, emptyAll,
} = useTrash();

/** 格式化删除时间为 yyyy-mm-dd */
function fmtDate(iso: string): string {
  return iso ? iso.slice(0, 10) : "";
}
</script>

<template>
  <div class="trash-view">
    <!-- 顶部工具栏 -->
    <div class="trash-toolbar">
      <span class="trash-title">回收站</span>
      <span class="trash-count">{{ items.length }}</span>
      <div class="toolbar-spacer" />
      <button class="act-btn" title="刷新" @click="load"><ReloadOutlined /></button>
      <button
        class="act-btn danger"
        :disabled="!items.length"
        @click="emptyAll"
      >
        <DeleteOutlined /> 清空回收站
      </button>
    </div>

    <!-- 列表面板 -->
    <section class="panel">
      <header class="panel-head">
        <span class="head-title">已删除书签</span>
        <span class="trash-count">{{ filtered.length }}</span>
      </header>

      <div class="panel-body" style="position:relative">
          <div v-for="it in filtered" :key="it.short_id" class="trash-item">
            <LinkOutlined class="item-type-icon bookmark" />
            <div class="item-main">
              <div class="item-title">{{ it.title || '(无标题)' }}</div>
              <div class="item-sub">
                <span class="type-tag">书签</span>
                <span v-if="it.category" class="sep">·</span>
                <span v-if="it.category">{{ it.category }}</span>
                <span class="sep">·</span>
                <span>删除于 {{ fmtDate(it.deleted_at) }}</span>
              </div>
            </div>
            <div class="item-actions">
              <button class="act-btn" title="恢复到原位置" @click="restore(it)">
                <UndoOutlined /> 恢复
              </button>
              <button class="act-btn danger" title="彻底删除（不可恢复）" @click="purge(it)">
                <DeleteOutlined /> 删除
              </button>
            </div>
          </div>

          <EmptyState
            v-if="!filtered.length && !loading"
            :icon="DeleteOutlined"
            text="回收站为空"
            size="small"
            inline
          />
      </div>
    </section>
  </div>
</template>
