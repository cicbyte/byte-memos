/** 前端类型定义（byte-link 书签应用） */

export interface StatsOverview {
  bookmark_count: number;
  tag_count: number;
  category_count: number;
  favorited_count: number;
  trashed_count: number;
  total_size: number;
}

// ============ 书签（byte-link 核心领域）============

export interface Bookmark {
  short_id: string;
  title: string;
  /** 书签地址（活跃行内唯一） */
  url: string;
  description: string;
  category_id: number | null;
  category: string | null;
  /** 本地缓存 favicon 文件相对路径（空串=未缓存） */
  favicon_path: string;
  visit_count: number;
  /** 是否归档（不常用但保留，区别于回收站） */
  is_archived: boolean;
  is_favorited: boolean;
  created_at: string;
  updated_at: string;
  tags: string[];
}

export interface BookmarkSearchHit {
  short_id: string;
  title: string;
  url: string;
  snippet: string;
  rank: number;
}

export interface Category {
  id: number;
  name: string;
  bookmark_count: number;
}

/** 兜底搜索命中（LIKE 兜底时使用） */
export interface SearchHit {
  short_id: string;
  title: string;
  snippet: string;
  rank: number;
  matched_tags: string[];
}

/** 统一搜索命中（search_all 返回，item_type 恒为 bookmark） */
export interface UnifiedSearchHit {
  short_id: string;
  item_type: ItemType;
  title: string;
  /** 命中上下文（FTS 高亮片段 / LIKE 兜底时为空） */
  snippet: string;
  category: string | null;
  /** 相关度（FTS bm25 负数越小越相关；LIKE 兜底为 0） */
  rank: number;
}

export interface TagStat {
  tag: string;
  count: number;
}

export interface ListParams {
  tag?: string;
  category?: string;
  favorited_only?: boolean;
  limit?: number;
  offset?: number;
}

/** 分页结果（list_*_paged 命令返回） */
export interface PagedResult<T> {
  items: T[];
  total: number;
}

// ============ 回收站 ============

export type ItemType = "bookmark";

export interface TrashedItem {
  short_id: string;
  item_type: ItemType;
  title: string;
  description: string;
  category: string | null;
  /** 删除时间（ISO8601） */
  deleted_at: string;
  /** 创建时间（ISO8601） */
  created_at: string;
}
