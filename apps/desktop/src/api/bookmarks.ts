import { invoke } from "@tauri-apps/api/core";
import type { Bookmark, BookmarkSearchHit, PagedResult } from "./types";

export interface BookmarkListParams {
  category?: string;
  tag?: string;
  /** 仅归档 / 仅活跃；不传 = 全部 */
  archived?: boolean;
  favorited_only?: boolean;
  limit?: number;
  offset?: number;
}

export const bookmarksApi = {
  // ===== 书签（byte-link 核心领域）=====
  addBookmark: (params: {
    url: string;
    title?: string;
    description?: string;
    category?: string;
    tags?: string[];
  }) =>
    invoke<Bookmark>("add_bookmark", {
      url: params.url,
      title: params.title ?? null,
      description: params.description ?? null,
      category: params.category ?? null,
      tags: params.tags ?? null,
    }),

  listBookmarks: (params: BookmarkListParams = {}) =>
    invoke<Bookmark[]>("list_bookmarks", {
      category: params.category ?? null,
      tag: params.tag ?? null,
      archived: params.archived ?? null,
      favoritedOnly: params.favorited_only ?? false,
      limit: params.limit ?? null,
      offset: params.offset ?? null,
    }),

  listBookmarksPaged: (params: BookmarkListParams = {}) =>
    invoke<PagedResult<Bookmark>>("list_bookmarks_paged", {
      category: params.category ?? null,
      tag: params.tag ?? null,
      archived: params.archived ?? null,
      favoritedOnly: params.favorited_only ?? false,
      limit: params.limit ?? null,
      offset: params.offset ?? null,
    }),

  getBookmark: (shortId: string) => invoke<Bookmark>("get_bookmark", { shortId }),

  /** 编辑用更新：传完整表单字段（覆盖式） */
  updateBookmark: (
    shortId: string,
    params: {
      title: string;
      url: string;
      description: string;
      category?: string;
      tags: string[];
      archived: boolean;
    },
  ) =>
    invoke<Bookmark>("update_bookmark", {
      shortId,
      title: params.title,
      url: params.url,
      description: params.description,
      category: params.category ?? null,
      tags: params.tags,
      archived: params.archived,
    }),

  deleteBookmark: (shortId: string) => invoke<void>("delete_bookmark", { shortId }),

  toggleBookmarkFavorite: (shortId: string) =>
    invoke<Bookmark>("toggle_bookmark_favorite", { shortId }),

  setBookmarkArchived: (shortId: string, archived: boolean) =>
    invoke<Bookmark>("set_bookmark_archived", { shortId, archived }),

  searchBookmarks: (query: string, limit = 20) =>
    invoke<BookmarkSearchHit[]>("search_bookmarks", { query, limit }),

  /** 记录一次访问（打开浏览器时调用） */
  incrementBookmarkVisit: (shortId: string) =>
    invoke<void>("increment_bookmark_visit", { shortId }),

  restoreBookmark: (shortId: string) => invoke<void>("restore_bookmark", { shortId }),

  purgeBookmark: (shortId: string) => invoke<void>("purge_bookmark", { shortId }),

  /** 抓取并缓存 favicon（成功返回相对路径，失败返回 null；不抛错） */
  fetchFavicon: (shortId: string) =>
    invoke<string | null>("fetch_bookmark_favicon", { shortId }),
};
