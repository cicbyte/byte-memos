/**
 * 回收站页逻辑（备忘录）
 *
 * 展示已软删除的备忘录，支持：
 * - 恢复（restoreMemo → 清 deleted_at 标记 + 重建 FTS）
 * - 彻底删除（purgeMemo → 物理删除 DB 行 + 关联，不可恢复）
 * - 清空回收站（emptyTrash → 对所有条目执行 purge）
 */
import { ref, computed, onMounted } from "vue";
import { message, Modal } from "ant-design-vue";
import { api } from "@/api";
import type { TrashedItem } from "@/api/types";
import { useLayoutStore } from "@/stores/layout";

export function useTrash() {
  const layout = useLayoutStore();
  const items = ref<TrashedItem[]>([]);
  const loading = ref(false);

  /** 按筛选条件过滤后的列表（备忘录回收站仅备忘录一类，直接返回全部） */
  const filtered = computed(() => items.value);

  function updateFooter() {
    layout.setStatusItems([
      { key: "view", text: "回收站" },
      { key: "count", text: `共 ${items.value.length} 项` },
    ]);
  }

  async function load() {
    loading.value = true;
    try {
      items.value = await api.listTrashed();
      updateFooter();
    } catch (e: any) {
      message.error(typeof e === "string" ? e : "加载回收站失败");
    } finally {
      loading.value = false;
    }
  }

  /** 恢复单个条目 */
  async function restore(it: TrashedItem) {
    try {
      await api.restoreMemo(it.short_id);
      message.success("已恢复");
      items.value = items.value.filter((x) => x.short_id !== it.short_id);
      updateFooter();
    } catch (e: any) {
      message.error(typeof e === "string" ? e : "恢复失败");
    }
  }

  /** 彻底删除单个条目（不可恢复，二次确认） */
  function purge(it: TrashedItem) {
    Modal.confirm({
      title: "彻底删除",
      content: `彻底删除「${it.title}」后无法恢复，确认删除？`,
      okType: "danger",
      okText: "彻底删除",
      cancelText: "取消",
      onOk: async () => {
        try {
          await api.purgeMemo(it.short_id);
          message.success("已彻底删除");
          items.value = items.value.filter((x) => x.short_id !== it.short_id);
          updateFooter();
        } catch (e: any) {
          message.error(typeof e === "string" ? e : "删除失败");
        }
      },
    });
  }

  /** 清空回收站（全部彻底删除，二次确认） */
  function emptyAll() {
    if (!items.value.length) return;
    Modal.confirm({
      title: "清空回收站",
      content: `将彻底删除回收站中的全部 ${items.value.length} 项内容，此操作不可恢复。确认清空？`,
      okType: "danger",
      okText: "清空",
      cancelText: "取消",
      onOk: async () => {
        try {
          await api.emptyTrash();
          message.success("回收站已清空");
          items.value = [];
          updateFooter();
        } catch (e: any) {
          message.error(typeof e === "string" ? e : "清空失败");
          await load();
        }
      },
    });
  }

  onMounted(load);

  return {
    items, loading, filtered,
    load, restore, purge, emptyAll,
  };
}
