import { invoke } from "@tauri-apps/api/core";
import type { Category } from "./types";

export const categoriesApi = {
  // ===== 分类管理 =====
  listCategories: () => invoke<Category[]>("list_categories"),

  createCategory: (name: string) => invoke<number>("create_category", { name }),

  renameCategory: (id: number, name: string) =>
    invoke<void>("rename_category", { id, name }),

  deleteCategory: (id: number) => invoke<void>("delete_category", { id }),

  /** 设置分类主题色（null 清除） */
  setCategoryColor: (id: number, color: string | null) =>
    invoke<void>("set_category_color", { id, color }),
};
