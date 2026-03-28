/**
 * 备忘录 AI 动作定义（与 core ai/prompts.rs ACTIONS 对应）
 *
 * - mode=text：流式产出 Markdown 文本 → 替换 / 追加 / 另存为新备忘录
 * - mode=tags / category：JSON 输出 → chip 采纳
 */
export interface AiActionDef {
  action: string;
  label: string;
  mode: "text" | "tags" | "category";
}

export const AI_ACTIONS: AiActionDef[] = [
  { action: "format", label: "整理成笔记", mode: "text" },
  { action: "summary", label: "生成摘要", mode: "text" },
  { action: "polish", label: "润色", mode: "text" },
  { action: "continue", label: "续写", mode: "text" },
  { action: "expand", label: "扩写", mode: "text" },
  { action: "todos", label: "提取待办", mode: "text" },
  { action: "tags", label: "推荐标签", mode: "tags" },
  { action: "category", label: "推荐分类", mode: "category" },
];

/** 翻译目标语言（二级菜单） */
export const TRANSLATE_LANGS = ["英文", "中文", "日文"];

export function findAction(action: string, lang = ""): AiActionDef {
  if (action === "translate") {
    return { action: "translate", label: `翻译为${lang || "英文"}`, mode: "text" };
  }
  return AI_ACTIONS.find((a) => a.action === action) ?? AI_ACTIONS[0];
}

/** 从 AI 产出中提取「# 标题行」；无标题行返回 null，正文为去除标题行后的内容 */
export function splitAiTitle(result: string): { title: string | null; body: string } {
  const lines = result.replace(/^\uFEFF/, "").split("\n");
  let i = 0;
  while (i < lines.length && lines[i].trim() === "") i++;
  const m = i < lines.length ? /^#\s+(.{1,80})\s*$/.exec(lines[i]) : null;
  if (m) {
    const rest = [...lines.slice(0, i), ...lines.slice(i + 1)].join("\n").replace(/^\n+/, "");
    return { title: m[1].trim(), body: rest };
  }
  return { title: null, body: result };
}

/** 宽松解析 JSON 数组（模型可能带前后缀说明文字） */
export function parseAiTags(result: string): string[] {
  const s = result.trim();
  const start = s.indexOf("[");
  const end = s.lastIndexOf("]");
  if (start >= 0 && end > start) {
    try {
      const arr = JSON.parse(s.slice(start, end + 1)) as unknown;
      if (Array.isArray(arr)) {
        return arr
          .filter((x): x is string => typeof x === "string")
          .map((x) => x.trim())
          .filter(Boolean)
          .slice(0, 5);
      }
    } catch {
      /* 落到行分割 */
    }
  }
  return s
    .split("\n")
    .map((l) => l.trim().replace(/^[-•·,，]\s*/, "").replace(/["']/g, ""))
    .filter((l) => l && !l.startsWith("[") && !l.startsWith("]"))
    .slice(0, 5);
}

/** 宽松解析 {category: "..."} */
export function parseAiCategory(result: string): string {
  const s = result.trim();
  const start = s.indexOf("{");
  const end = s.lastIndexOf("}");
  if (start >= 0 && end > start) {
    try {
      const obj = JSON.parse(s.slice(start, end + 1)) as { category?: unknown };
      if (typeof obj.category === "string") return obj.category.trim();
    } catch {
      /* 落到单行 */
    }
  }
  return s.split("\n").map((l) => l.trim()).find((l) => l) ?? "";
}

/** 「AI 整理」产出：标题 / 标签 / 分类 三元组 */
export interface AiOrganize {
  title: string;
  tags: string[];
  category: string;
}

/** 宽松解析 {title, tags, category}（模型可能带前后缀说明文字；全空视为失败） */
export function parseOrganize(result: string): AiOrganize {
  const s = result.trim();
  const start = s.indexOf("{");
  const end = s.lastIndexOf("}");
  if (start >= 0 && end > start) {
    try {
      const o = JSON.parse(s.slice(start, end + 1)) as {
        title?: unknown;
        tags?: unknown;
        category?: unknown;
      };
      const str = (v: unknown) => (typeof v === "string" ? v.trim() : "");
      const tags = Array.isArray(o.tags)
        ? o.tags
            .filter((x): x is string => typeof x === "string")
            .map((x) => x.trim())
            .filter(Boolean)
            .slice(0, 3)
        : [];
      return {
        title: str(o.title).slice(0, 120),
        tags,
        category: str(o.category),
      };
    } catch {
      /* 落到全空 */
    }
  }
  return { title: "", tags: [], category: "" };
}
