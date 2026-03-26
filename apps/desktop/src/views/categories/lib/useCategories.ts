/**
 * 分类页 composable —— 状态 + 逻辑（书签专用）
 *
 * 持有分类列表、选中态、预览数据、新建/重命名/删除等全部响应式状态，
 * 以及对应的 API 调用与交互函数。主入口组合后通过 props 下发子组件。
 */
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { message, Modal } from "ant-design-vue";
import { api } from "@/api";
import type { Category, Bookmark } from "@/api/types";
import { useLayoutStore } from "@/stores/layout";

export function useCategories() {
  const router = useRouter();
  const layout = useLayoutStore();

  const categories = ref<Category[]>([]);
  const selectedId = ref<number | null>(null);
  const loading = ref(false);

  // 选中分类的书签预览
  const previewBookmarks = ref<Bookmark[]>([]);
  const previewLoading = ref(false);
  const detailTab = ref<string>("overview");

  // 新建
  const newName = ref("");
  const creating = ref(false); // 输入框显示态（默认只显示"新建分类"按钮）
  // 行内重命名
  const editingId = ref<number | null>(null);
  const editingName = ref("");

  const selected = computed(() =>
    categories.value.find((c) => c.id === selectedId.value) ?? null
  );

  const tabs = computed(() =>
    selected.value
      ? [
          { key: "overview", label: "概览" },
          { key: "bookmarks", label: "书签", count: selected.value.bookmark_count },
        ]
      : []
  );

  async function load() {
    loading.value = true;
    try {
      categories.value = await api.listCategories();
      layout.setStatusItems([
        { key: "view", text: "分类" },
        { key: "count", text: `${categories.value.length} 个` },
      ]);
    } finally {
      loading.value = false;
    }
  }

  function selectCat(c: Category) {
    selectedId.value = c.id;
    editingId.value = null;
    detailTab.value = "overview";
    loadPreviews(c.name);
  }

  /** 拉取该分类下的书签 */
  async function loadPreviews(catName: string) {
    previewLoading.value = true;
    try {
      previewBookmarks.value = await api.listBookmarks({ category: catName, limit: 50 });
    } catch {
      previewBookmarks.value = [];
    } finally {
      previewLoading.value = false;
    }
  }

  function startCreate() {
    newName.value = "";
    creating.value = true;
  }

  function cancelCreate() {
    creating.value = false;
    newName.value = "";
  }

  async function onCreate() {
    const name = newName.value.trim();
    if (!name) {
      creating.value = false;
      return;
    }
    if (categories.value.some((c) => c.name === name)) {
      message.warning("分类名已存在");
      return;
    }
    try {
      const id = await api.createCategory(name);
      message.success("已创建");
      newName.value = "";
      creating.value = false;
      await load();
      selectedId.value = id;
    } catch (e: any) {
      message.error(typeof e === "string" ? e : "创建失败");
    }
  }

  function startEdit(c: Category) {
    editingId.value = c.id;
    editingName.value = c.name;
  }

  async function commitEdit() {
    if (editingId.value === null) return;
    const name = editingName.value.trim();
    if (!name || name === selected.value?.name) {
      editingId.value = null;
      return;
    }
    try {
      await api.renameCategory(editingId.value, name);
      message.success("已重命名");
      editingId.value = null;
      await load();
    } catch (e: any) {
      message.error(typeof e === "string" ? e : "重命名失败");
    }
  }

  function cancelEdit() {
    editingId.value = null;
  }

  function onRemove(c: Category) {
    const total = c.bookmark_count;
    Modal.confirm({
      title: "删除分类",
      content:
        total > 0
          ? `分类「${c.name}」下有 ${total} 个书签。删除后这些书签保留，仅解除分类绑定。确认删除？`
          : `确认删除空分类「${c.name}」？`,
      okText: "删除",
      okType: "danger",
      cancelText: "取消",
      onOk: async () => {
        try {
          await api.deleteCategory(c.id);
          message.success("已删除");
          if (selectedId.value === c.id) selectedId.value = null;
          await load();
        } catch (e: any) {
          message.error(typeof e === "string" ? e : "删除失败");
        }
      },
    });
  }

  /** 跳转到书签页（bookmarks 页暂不支持 category query，仅跳列表） */
  function goBookmarks() {
    router.push("/bookmarks");
  }

  // 全局 ESC：输入框失焦后也能收起（新建分类 / 行内重命名）
  function handleEsc(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    if (creating.value) {
      cancelCreate();
    } else if (editingId.value !== null) {
      cancelEdit();
    }
  }

  return {
    // state
    categories,
    selectedId,
    loading,
    previewBookmarks,
    previewLoading,
    detailTab,
    newName,
    creating,
    editingId,
    editingName,
    // computed
    selected,
    tabs,
    // functions
    load,
    selectCat,
    loadPreviews,
    startCreate,
    cancelCreate,
    onCreate,
    startEdit,
    commitEdit,
    cancelEdit,
    onRemove,
    goBookmarks,
    handleEsc,
  };
}
