import { invoke } from "@tauri-apps/api/core";
import type { TrashedItem } from "./types";

export const trashApi = {
  restoreDiagram: (shortId: string) => invoke<void>("restore_diagram", { shortId }),
  purgeDiagram: (shortId: string) => invoke<void>("purge_diagram", { shortId }),
  restoreTemplate: (shortId: string) => invoke<void>("restore_template", { shortId }),
  purgeTemplate: (shortId: string) => invoke<void>("purge_template", { shortId }),

  // ============ 回收站 ============

  /** 列出回收站全部条目（三类合并，按删除时间降序） */
  listTrashed: () => invoke<TrashedItem[]>("list_trashed"),

  /** 从回收站恢复项目 */
  restoreProject: (shortId: string) =>
    invoke<void>("restore_project", { shortId }),

  /** 从回收站恢复片段 */
  restoreSnippet: (shortId: string) =>
    invoke<void>("restore_snippet", { shortId }),

  /** 从回收站恢复笔记 */
  restoreNote: (shortId: string) => invoke<void>("restore_note", { shortId }),

  /** 彻底删除项目（不可恢复） */
  purgeProject: (shortId: string) =>
    invoke<void>("purge_project", { shortId }),

  /** 彻底删除片段（不可恢复） */
  purgeSnippet: (shortId: string) =>
    invoke<void>("purge_snippet", { shortId }),

  /** 彻底删除笔记（不可恢复） */
  purgeNote: (shortId: string) => invoke<void>("purge_note", { shortId }),

  /** 清空回收站（全部彻底删除） */
  emptyTrash: () => invoke<void>("empty_trash"),
};
