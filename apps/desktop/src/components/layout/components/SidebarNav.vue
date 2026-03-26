<script setup lang="ts">
/**
 * Sidebar 主体导航（平铺模式，参考 byte-writer）。
 *
 * 全部菜单项常显直达：小标题（side-label 风格的分组标签）仅作视觉分隔，
 * 不折叠——10 个菜单项一屏放得下，分组折叠只增加点击成本且收起时
 * 当前路由高亮不可见。折叠态为纯图标列（小标题隐藏，tooltip 补全名）。
 */
import {
  menuGroups,
  topItems,
  labelOf,
} from "../lib/constants";

defineProps<{
  collapsed: boolean;
  currentPath: string;
}>();

const emit = defineEmits<{
  (e: "navigate", key: string): void;
}>();
</script>

<template>
  <nav class="sidebar-nav">
    <!-- 顶层独立项（仪表盘） -->
    <template v-if="!collapsed">
      <div
        v-for="item in topItems"
        :key="item.key"
        class="nav-item top-item"
        :class="{ active: currentPath === item.key }"
        @click="emit('navigate', item.key)"
      >
        <component :is="item.icon" class="nav-icon" />
        <span class="nav-label">{{ labelOf[item.key] }}</span>
      </div>
    </template>
    <template v-else>
      <div
        v-for="item in topItems"
        :key="item.key"
        class="nav-item top-item-collapsed"
        :class="{ active: currentPath === item.key }"
        :title="labelOf[item.key]"
        @click="emit('navigate', item.key)"
      >
        <component :is="item.icon" class="nav-icon" />
      </div>
    </template>

    <!-- 分组平铺：小标题仅视觉分隔，子项常显直达 -->
    <template v-for="group in menuGroups" :key="group.key">
      <div v-if="!collapsed" class="nav-group">
        <div class="nav-label-title">{{ group.label }}</div>
        <div
          v-for="child in group.children"
          :key="child.key"
          class="nav-item"
          :class="{ active: currentPath === child.key }"
          @click="emit('navigate', child.key)"
        >
          <component :is="child.icon" class="nav-icon" />
          <span class="nav-label">{{ labelOf[child.key] }}</span>
        </div>
      </div>
      <template v-else>
        <div
          v-for="child in group.children"
          :key="child.key"
          class="nav-item nav-item-collapsed"
          :class="{ active: currentPath === child.key }"
          :title="labelOf[child.key]"
          @click="emit('navigate', child.key)"
        >
          <component :is="child.icon" class="nav-icon" />
        </div>
      </template>
    </template>
  </nav>
</template>

<style scoped>
.sidebar-nav {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-sm);
}
.sidebar-nav::-webkit-scrollbar {
  width: 5px;
}
.sidebar-nav::-webkit-scrollbar-thumb {
  background: var(--border-base);
  border-radius: 3px;
}

/* 顶层独立项（比普通菜单项略重） */
.nav-item.top-item {
  padding: 9px 12px;
  font-weight: 500;
}
.nav-item.top-item-collapsed {
  justify-content: center;
  padding: 10px;
  margin-bottom: 2px;
}

/* 分组小标题：仅视觉分隔，不可点击 */
.nav-group {
  margin-bottom: 2px;
}
.nav-label-title {
  padding: 14px 10px 4px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  letter-spacing: 0.05em;
  user-select: none;
  white-space: nowrap;
}
.nav-group:first-of-type .nav-label-title {
  padding-top: 6px;
}

/* 折叠态：隐藏小标题，子项并入图标列 */
.nav-item-collapsed {
  justify-content: center;
  padding: 10px;
}

/* ===== 菜单项 ===== */
/* 导航条目：30px 行高胶囊、低透明度层次（悬浮轻于选中）、120ms 无位移（柔和无感） */
.nav-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  height: 30px;
  padding: 0 8px;
  margin-bottom: 2px;
  border-radius: 8px;
  cursor: pointer;
  transition: background-color 120ms ease, color 120ms ease;
  color: var(--text-secondary);
  font-size: 13px;
  white-space: nowrap;
  user-select: none;
}
.nav-item:hover {
  background: var(--color-nav-hover);
  color: var(--text-primary);
}
.nav-item.active {
  background: var(--color-nav-active);
  color: var(--text-primary);
  font-weight: 550;
}
.nav-icon {
  font-size: 15px;
  color: var(--text-tertiary);
  flex-shrink: 0;
  transition: color 120ms ease;
}
.nav-item:hover .nav-icon,
.nav-item.active .nav-icon {
  color: var(--text-primary);
}

.nav-label {
  font-size: 13px;
}
</style>
