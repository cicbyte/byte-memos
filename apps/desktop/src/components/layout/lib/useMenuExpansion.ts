/**
 * Sidebar 菜单展开/折叠状态逻辑
 *
 * expanded 为组件内响应式状态，由调用方（Sidebar.vue 主入口）持有唯一实例，
 * 通过 props 下发，避免多处副本。
 */
import { ref, watch } from "vue";
import { useRoute } from "vue-router";
import { groupOf } from "./constants";

export function useMenuExpansion() {
  const route = useRoute();

  const expanded = ref<Set<string>>(
    new Set([groupOf(route.path)].filter(Boolean) as string[])
  );

  watch(
    () => route.path,
    (p) => {
      const g = groupOf(p);
      if (g) {
        const next = new Set(expanded.value);
        next.add(g);
        expanded.value = next;
      }
    },
    { immediate: true }
  );

  function toggleGroup(key: string) {
    const next = new Set(expanded.value);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    expanded.value = next;
  }

  function isExpanded(key: string) {
    return expanded.value.has(key);
  }

  return { expanded, toggleGroup, isExpanded };
}
