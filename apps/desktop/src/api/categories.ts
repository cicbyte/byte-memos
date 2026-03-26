import { invoke } from "@tauri-apps/api/core";
import type { Category } from "./types";

export const categoriesApi = {
  // ===== 分类管理 =====
  listCategories: () => invoke<Category[]>("list_categories"),

  createCategory: (name: string) => invoke<number>("create_category", { name }),

  renameCategory: (id: number, name: string) =>
    invoke<void>("rename_category", { id, name }),

  deleteCategory: (id: number) => invoke<void>("delete_category", { id }),
};
