import { invoke } from "@tauri-apps/api/core";
import type { TrashedItem } from "./types";

export const trashApi = {
  /** 列出回收站全部条目（按删除时间降序） */
  listTrashed: () => invoke<TrashedItem[]>("list_trashed"),

  /** 从回收站恢复备忘录 */
  restoreMemo: (shortId: string) => invoke<void>("restore_memo", { shortId }),

  /** 彻底删除备忘录（不可恢复） */
  purgeMemo: (shortId: string) => invoke<void>("purge_memo", { shortId }),

  /** 清空回收站（全部彻底删除） */
  emptyTrash: () => invoke<void>("empty_trash"),
};
