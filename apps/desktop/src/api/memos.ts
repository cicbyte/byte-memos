/**
 * 备忘录 API（byte-memos 核心领域）
 *
 * 封装 src-tauri/src/commands/memos.rs 的 11 个命令 + stats.rs 的统计概览。
 * invoke 参数键用 camelCase（Tauri 自动映射 Rust snake_case）。
 */
import { invoke } from "@tauri-apps/api/core";
import type { Memo, MemoListParams, MemoSearchHit, MemoVersionInfo, PagedResult, StatsOverview } from "./types";

export const memosApi = {
  // ===== 备忘录 =====

  /** 新增备忘录（visibility 默认 PRIVATE，tags/category 可选） */
  addMemo: (input: { title: string; content: string; visibility?: string; category?: string | null; tags?: string[] }) =>
    invoke<Memo>("add_memo", {
      title: input.title,
      content: input.content,
      visibility: input.visibility ?? null,
      category: input.category ?? null,
      tags: input.tags ?? [],
    }),

  /** 编辑备忘录（覆盖式：传完整表单字段） */
  updateMemo: (shortId: string, form: { title: string; content: string; visibility?: string | null; category?: string | null; tags: string[]; pinned: boolean }) =>
    invoke<Memo>("update_memo", {
      shortId,
      title: form.title,
      content: form.content,
      visibility: form.visibility ?? null,
      category: form.category ?? null,
      tags: form.tags,
      pinned: form.pinned,
    }),

  /** 列出备忘录（过滤 + 分页） */
  listMemos: (params: MemoListParams = {}) =>
    invoke<Memo[]>("list_memos", {
      category: params.category ?? null,
      tag: params.tag ?? null,
      pinnedOnly: params.pinned_only ?? false,
      limit: params.limit ?? null,
      offset: params.offset ?? null,
    }),

  /** 分页列出备忘录（items + total，供无限滚动） */
  listMemosPaged: (params: MemoListParams = {}) =>
    invoke<PagedResult<Memo>>("list_memos_paged", {
      category: params.category ?? null,
      tag: params.tag ?? null,
      pinnedOnly: params.pinned_only ?? false,
      limit: params.limit ?? null,
      offset: params.offset ?? null,
    }),

  /** 读取备忘录详情 */
  getMemo: (shortId: string) => invoke<Memo>("get_memo", { shortId }),

  /** 软删除（移入回收站，可恢复） */
  deleteMemo: (shortId: string) => invoke<void>("delete_memo", { shortId }),

  /** 置顶/取消置顶（切换） */
  toggleMemoPinned: (shortId: string) => invoke<Memo>("toggle_memo_pinned", { shortId }),

  /** 设置可见性（PRIVATE / PROTECTED / PUBLIC） */
  setMemoVisibility: (shortId: string, visibility: string) =>
    invoke<Memo>("set_memo_visibility", { shortId, visibility }),

  /** 全文搜索（FTS5 + 中文 LIKE 兜底） */
  searchMemos: (query: string, limit = 20) =>
    invoke<MemoSearchHit[]>("search_memos", { query, limit }),

  /** 从回收站恢复 */
  restoreMemo: (shortId: string) => invoke<void>("restore_memo", { shortId }),

  /** 彻底删除（不可恢复） */
  purgeMemo: (shortId: string) => invoke<void>("purge_memo", { shortId }),

  // ===== 版本历史 =====

  /** 版本历史（内容变更自动快照，按时间倒序） */
  memoHistory: (shortId: string) =>
    invoke<MemoVersionInfo[]>("memo_history", { shortId }),

  /** 回滚到指定版本（生成新版本记录，历史只增不减） */
  restoreMemoVersion: (shortId: string, versionId: number) =>
    invoke<Memo>("restore_memo_version", { shortId, versionId }),

  /** 单篇导出并落盘（format: markdown / html），返回写入路径 */
  exportMemo: (shortId: string, format: "markdown" | "html", path: string) =>
    invoke<string>("export_memo", { shortId, format, path }),

  // ===== 统计 =====

  /** 统计概览（备忘录/标签/分类计数 + 存储占用 + 置顶/回收站计数） */
  statsOverview: () => invoke<StatsOverview>("stats_overview"),
};
