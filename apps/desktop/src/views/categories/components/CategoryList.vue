<script setup lang="ts">
/**
 * 分类列表（categories → 左 Panel）
 *
 * 可变状态（newName/editingName）通过 defineModel 与主入口双向绑定，
 * 状态唯一来源仍在主入口的 useCategories；本组件不持有同名状态副本。
 * 新建/选中/编辑/删除以事件上报，由主入口调用 composable 函数。
 */
import { ref, watch, nextTick } from "vue";
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  FolderOutlined,
} from "@ant-design/icons-vue";
import EmptyState from "@/components/common/EmptyState.vue";
import Panel from "@/components/shared/Panel.vue";
import type { Category } from "@/api/types";

const props = defineProps<{
  categories: Category[];
  loading: boolean;
  selectedId: number | null;
  creating: boolean;
  editingId: number | null;
}>();

const newName = defineModel<string>("newName", { required: true });
const editingName = defineModel<string>("editingName", { required: true });

const emit = defineEmits<{
  startCreate: [];
  cancelCreate: [];
  create: [];
  select: [c: Category];
  startEdit: [c: Category];
  commitEdit: [];
  cancelEdit: [];
  remove: [c: Category];
}>();

// 新建输入框 ref —— 等输入框渲染后自动聚焦
const createInputRef = ref();

// creating 变 true 时自动聚焦输入框（原 startCreate 内的 nextTick 逻辑）
watch(
  () => props.creating,
  (val) => {
    if (val) {
      nextTick(() => {
        const el = createInputRef.value?.$el?.querySelector("input");
        el?.focus();
      });
    }
  },
);
</script>

<template>
  <Panel title="分类" :count="categories.length">
    <!-- 新建入口（默认按钮，点击变输入框） -->
    <div class="create-zone">
      <div v-if="!creating" class="create-btn" @click="emit('startCreate')">
        <PlusOutlined />
        <span>新建分类</span>
      </div>
      <div v-else class="create-input-row">
        <a-input
          ref="createInputRef"
          v-model:value="newName"
          placeholder="输入分类名，回车确认"
          allow-clear
          size="small"
          @press-enter="emit('create')"
          @keydown.esc="emit('cancelCreate')"
        />
      </div>
    </div>

    <div class="panel-body">
      <div
        v-for="c in categories"
        :key="c.id"
        class="cat-row"
        :class="{ active: selectedId === c.id }"
        @click="emit('select', c)"
        @dblclick="emit('startEdit', c)"
      >
        <template v-if="editingId === c.id">
          <a-input
            v-model:value="editingName"
            size="small"
            autofocus
            @press-enter="emit('commitEdit')"
            @keydown.esc="emit('cancelEdit')"
            @click.stop
          />
          <div class="edit-actions">
            <a-button size="small" type="primary" @click.stop="emit('commitEdit')">确定</a-button>
            <a-button size="small" @click.stop="emit('cancelEdit')">取消</a-button>
          </div>
        </template>
        <template v-else>
          <FolderOutlined class="row-icon" />
          <span class="row-name">{{ c.name }}</span>
          <div class="row-actions">
            <button class="act-btn" title="重命名" @click.stop="emit('startEdit', c)">
              <EditOutlined />
            </button>
            <button class="act-btn danger" title="删除" @click.stop="emit('remove', c)">
              <DeleteOutlined />
            </button>
          </div>
        </template>
      </div>
    </div>

    <template #empty>
      <EmptyState
        v-if="!categories.length && !loading"
        :icon="FolderOutlined"
        text="暂无分类"
        hint="在上方输入名称创建"
      />
    </template>
  </Panel>
</template>

<style scoped>
/* ===== 新建入口（按钮 ↔ 输入框切换） ===== */
.create-zone {
  padding: var(--spacing-sm);
  border-bottom: 1px solid var(--border-light);
  flex-shrink: 0;
}
.create-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px;
  border: 1px dashed var(--border-base);
  border-radius: var(--radius-md);
  color: var(--text-tertiary);
  font-size: 13px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.create-btn:hover {
  border-color: var(--color-primary-border);
  color: var(--color-primary);
  background: var(--color-primary-bg);
}

/* ===== 左列表项 ===== */
.cat-row {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--transition-fast);
  /* 不换行：名称超长走 ellipsis，避免 hover 时 actions 显隐触发整行重排抖动 */
}
.cat-row:hover {
  background: var(--bg-elevated);
}
.cat-row.active {
  background: var(--color-primary-bg);
}
.cat-row.active::before {
  content: "";
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 3px;
  border-radius: 2px;
  background: var(--color-primary);
}
.row-icon {
  font-size: 14px;
  color: var(--text-tertiary);
  flex-shrink: 0;
}
.cat-row.active .row-icon {
  color: var(--color-primary);
}
.row-name {
  flex: 1;
  font-size: 13px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.row-actions {
  /* 始终占位，仅切换可见性，避免 hover 时 display 切换导致整行重排抖动 */
  display: flex;
  gap: 2px;
  flex-shrink: 0;
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
  transition: opacity var(--transition-fast);
}
.cat-row:hover .row-actions,
.cat-row.active .row-actions {
  opacity: 1;
  visibility: visible;
  pointer-events: auto;
}
.act-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  font-size: 12px;
  border-radius: var(--radius-xs);
}
.act-btn:hover {
  background: var(--bg-base);
  color: var(--text-primary);
}
.act-btn.danger:hover {
  color: var(--color-danger);
}
.edit-actions {
  display: flex;
  gap: 4px;
  width: 100%;
  margin-top: 4px;
}
</style>
