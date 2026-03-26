/**
 * 桌面端 API 封装：通过 Tauri invoke 调用 Rust 端 command（src-tauri/src/lib.rs）
 *
 * 每个 Rust #[tauri::command] 在此对应一个类型安全的包装。
 *
 * 实现按域拆分到 ./<domain>.ts，本文件仅做聚合：将各域子对象展开合并为单一 `api`，
 * 对外保持平铺方法名（api.addBookmark / api.listBookmarks / api.ping … 调用方式不变）。
 *
 * byte-link 为纯书签应用：仅保留 bookmarks/categories/trash/backup/wallpaper/
 * preferences/mcp-server/api-server/dialog 域。
 */
import { dialogApi } from "./dialog";
import { bookmarksApi } from "./bookmarks";
import { categoriesApi } from "./categories";
import { trashApi } from "./trash";
import { backupApi } from "./backup";
import { wallpaperApi } from "./wallpaper";
import { preferencesApi } from "./preferences";
import { mcpServerApi } from "./mcp-server";
import { apiServerApi } from "./api-server";

export const api = {
  ...dialogApi,
  ...bookmarksApi,
  ...categoriesApi,
  ...trashApi,
  ...backupApi,
  ...wallpaperApi,
  ...preferencesApi,
  ...mcpServerApi,
  ...apiServerApi,
};
