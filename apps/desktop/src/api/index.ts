/**
 * 桌面端 API 封装：通过 Tauri invoke 调用 Rust 端 command（src-tauri/src/lib.rs）
 *
 * 每个 Rust #[tauri::command] 在此对应一个类型安全的包装。
 *
 * 实现按域拆分到 ./<domain>.ts，本文件仅做聚合：将各域子对象展开合并为单一 `api`，
 * 对外保持平铺方法名（api.addMemo / api.listMemos / api.ping … 调用方式不变）。
 *
 * byte-memos 为备忘录应用：memos/categories/trash/backup/wallpaper/
 * preferences/mcp-server/api-server/dialog 域。
 */
import { dialogApi } from "./dialog";
import { memosApi } from "./memos";
import { categoriesApi } from "./categories";
import { attachmentsApi } from "./attachments";
import { trashApi } from "./trash";
import { backupApi } from "./backup";
import { wallpaperApi } from "./wallpaper";
import { preferencesApi } from "./preferences";
import { mcpServerApi } from "./mcp-server";
import { apiServerApi } from "./api-server";

export const api = {
  ...dialogApi,
  ...memosApi,
  ...categoriesApi,
  ...attachmentsApi,
  ...trashApi,
  ...backupApi,
  ...wallpaperApi,
  ...preferencesApi,
  ...mcpServerApi,
  ...apiServerApi,
};
