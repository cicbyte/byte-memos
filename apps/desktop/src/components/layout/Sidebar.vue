<script setup lang="ts">
/**
 * Sidebar 平铺导航（参考 byte-writer）
 *
 * 顶部 logo header（高度同 navbar）：左侧 logo+文字
 * 主体：平铺菜单（小标题仅视觉分隔，全部菜单项常显直达，路由联动高亮）
 * 折叠态：仅图标，tooltip 补全名
 *
 * 本文件为组合入口；静态菜单数据见 ./lib/constants。
 */
import { useRoute, useRouter } from "vue-router";
import { useLayoutStore } from "@/stores/layout";
import SidebarLogo from "./components/SidebarLogo.vue";
import SidebarNav from "./components/SidebarNav.vue";
import SidebarBottom from "./components/SidebarBottom.vue";

const route = useRoute();
const router = useRouter();
const layout = useLayoutStore();

function navigate(key: string) {
  router.push(key);
}
</script>

<template>
  <div class="sidebar" :class="{ collapsed: layout.sidebarCollapsed }">
    <!-- logo header：仅 logo + 文字（收缩按钮在 Navbar 左侧） -->
    <SidebarLogo :collapsed="layout.sidebarCollapsed" @home="navigate('/')" />

    <!-- 导航 -->
    <SidebarNav
      :collapsed="layout.sidebarCollapsed"
      :current-path="route.path"
      @navigate="navigate"
    />

    <!-- 底部 -->
    <SidebarBottom :collapsed="layout.sidebarCollapsed" />
  </div>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  position: relative;
  z-index: 2;
  /* 悬浮面板体系：底色/边框由 AppLayout 的 shell-panel 三件套统一提供 */
  transition: width var(--transition-normal);
  overflow: hidden;
  flex-shrink: 0;
  width: var(--sidebar-width-expanded);
}
.sidebar.collapsed {
  width: var(--sidebar-width);
}

/*
 * 折叠态对子组件根元素（.sidebar-logo / .sidebar-bottom）的微调。
 * 这两条规则依赖父级 .sidebar.collapsed 状态，故保留在主入口；
 * 子组件根元素会继承父级 scoped 属性，可被正确匹配。
 */
.sidebar.collapsed .sidebar-logo {
  justify-content: center;
  padding: 0;
}
.sidebar.collapsed .sidebar-bottom {
  justify-content: center;
}
</style>
