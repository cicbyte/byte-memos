/**
 * 工具步骤展示元数据（参考 ZCode ToolCallBlocks：每工具一个图标 + 类别词 + 参数摘要）
 * 图标为 Feather 自绘 SVG path（24 viewBox，描边风格与应用一致）
 */
export interface StepDisplay {
  /** Feather SVG 内部路径（外层 svg 由模板统一渲染） */
  icon: string;
  /** 类别动词（如 搜索/读取/创建） */
  kind: string;
  /** 参数摘要（如 「部署」/ ab3f9k21） */
  detail: string;
}

const P = {
  search:
    '<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>',
  file:
    '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>',
  list:
    '<line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/>',
  folder:
    '<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>',
  tag:
    '<path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.83z"/><line x1="7" y1="7" x2="7.01" y2="7"/>',
  chart:
    '<line x1="12" y1="20" x2="12" y2="10"/><line x1="18" y1="20" x2="18" y2="4"/><line x1="6" y1="20" x2="6" y2="16"/>',
  plus:
    '<rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><line x1="12" y1="8" x2="12" y2="16"/><line x1="8" y1="12" x2="16" y2="12"/>',
  edit:
    '<path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/>',
  trash:
    '<polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>',
} as const;

function cut(s: string, n: number): string {
  const t = s.trim();
  return t.length <= n ? t : `${t.slice(0, n)}…`;
}

/** 由工具名 + 参数 JSON 产出展示元数据 */
export function describeStep(name: string, argsJson: string): StepDisplay {
  let args: Record<string, unknown> = {};
  try {
    args = JSON.parse(argsJson) as Record<string, unknown>;
  } catch {
    /* 参数非 JSON 时仅显示类别词 */
  }
  const str = (k: string): string => (typeof args[k] === "string" ? (args[k] as string) : "");
  const num = (k: string): number | undefined =>
    typeof args[k] === "number" ? (args[k] as number) : undefined;

  switch (name) {
    case "search_memos":
      return { icon: P.search, kind: "搜索", detail: str("query") ? `「${cut(str("query"), 18)}」` : "" };
    case "list_memos": {
      const d = [
        str("category") && `分类 ${cut(str("category"), 10)}`,
        str("tag") && `标签 ${cut(str("tag"), 10)}`,
        args.pinned_only && "仅置顶",
      ]
        .filter(Boolean)
        .join(" · ");
      return { icon: P.list, kind: "列出备忘", detail: d || "最近" };
    }
    case "get_memo":
      return { icon: P.file, kind: "读取", detail: str("short_id") };
    case "list_categories":
      return { icon: P.folder, kind: "查看分类", detail: "" };
    case "list_tags":
      return { icon: P.tag, kind: "查看标签", detail: "" };
    case "daily_stats":
      return { icon: P.chart, kind: "统计", detail: `${num("days") ?? 30} 天` };
    case "create_memo":
      return {
        icon: P.plus,
        kind: "创建",
        detail: str("title") ? `「${cut(str("title"), 14)}」` : cut(str("content"), 14),
      };
    case "update_memo":
      return { icon: P.edit, kind: "更新", detail: str("short_id") };
    case "delete_memo":
      return { icon: P.trash, kind: "删除", detail: str("short_id") };
    default:
      return { icon: P.list, kind: name, detail: "" };
  }
}
