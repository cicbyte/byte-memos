/** 前端类型定义（byte-memos 备忘录应用）——与 crates/core/src/model 逐字段对齐 */

export interface StatsOverview {
  memo_count: number;
  tag_count: number;
  category_count: number;
  /** 数据目录磁盘占用（字节） */
  total_size: number;
  pinned_count: number;
  trashed_count: number;
}

// ============ 备忘录（byte-memos 核心领域）============

export interface Memo {
  short_id: string;
  title: string;
  /** Markdown 内容 */
  content: string;
  /** 可见性：PRIVATE / PROTECTED / PUBLIC */
  visibility: "PRIVATE" | "PROTECTED" | "PUBLIC" | string;
  /** 是否置顶 */
  pinned: boolean;
  category_id: number | null;
  /** 分类名（关联查询时填充） */
  category: string | null;
  tags: string[];
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

/** 备忘录版本快照（内容变更自动记录，历史只增不减） */
export interface MemoVersionInfo {
  id: number;
  memo_short_id: string;
  memo_title: string;
  content: string;
  created_at: string;
}

export interface MemoSearchHit {
  short_id: string;
  title: string;
  /** 内容片段（FTS 高亮） */
  snippet: string | null;
  category: string | null;
  tags: string[];
  rank: number | null;
}

export interface Category {
  id: number;
  name: string;
  /** 分类主题色（hex，null=未设置） */
  color: string | null;
  memo_count: number;
}

/** 统一搜索命中（search_all 返回，item_type 恒为 memo） */
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

/** 备忘录列表过滤参数（对齐 core MemoFilter） */
export interface MemoListParams {
  tag?: string;
  category?: string;
  /** 仅置顶 */
  pinned_only?: boolean;
  limit?: number;
  offset?: number;
}

/** 分页结果（list_*_paged 命令返回） */
export interface PagedResult<T> {
  items: T[];
  total: number;
}

// ============ 回收站 ============

export type ItemType = "memo";

// ============ 附件 ============

/** 附件记录（对齐 core db::attachments::Attachment） */
export interface AttachmentInfo {
  id: number;
  memo_short_id: string;
  filename: string;
  /** FILE 模式为相对 data 目录路径；DB 模式为空 */
  file_path: string;
  file_size: number;
  mime_type: string;
  /** DB / FILE */
  storage_type: string;
  created_at: string;
}

/** 附件总览行（资源库用，orphan=所属备忘录已不存在） */
export interface AttachmentOverview extends AttachmentInfo {
  orphan: boolean;
}

/** 附件内容载荷（attachment_content 命令返回，bytes 为字节数组） */
export interface AttachmentContent {
  filename: string;
  mime_type: string;
  bytes: number[];
}

/** 单日备忘录计数（日历热力图数据源，本地时区日期） */
export interface DailyCount {
  date: string;
  count: number;
}

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
