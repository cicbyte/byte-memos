/**
 * 布局 store（对齐 byte-link-桌面布局设计.md §8）
 * 只管纯布局：sidebar 折叠态 + statusbar 状态项。
 */
import { defineStore } from "pinia";
import { ref } from "vue";

export interface StatusItem {
  key: string;
  text: string;
}

export const useLayoutStore = defineStore("layout", () => {
  const sidebarCollapsed = ref(false);
  const statusItems = ref<StatusItem[]>([]);

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value;
  }

  function setStatusItems(items: StatusItem[]) {
    statusItems.value = items;
  }

  return { sidebarCollapsed, statusItems, toggleSidebar, setStatusItems };
});
