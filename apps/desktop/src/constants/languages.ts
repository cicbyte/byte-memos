/**
 * 标准语言列表（前端统一引用）
 *
 * 与 core/lang_ext.rs 的映射对齐。内置语言不可删除，自定义语言可增删。
 *
 * 每项 { value, label, builtin, highlight }：
 * - value：标准语言名（存入 DB）
 * - label：显示名
 * - builtin：true=内置语言（不可删除）
 * - highlight：false=Monaco 可能不支持语法高亮（标识给用户）
 */

export interface LangOption {
  value: string;
  label: string;
  builtin: boolean;
  highlight: boolean;
}

/** 内置语言（对齐 core/lang_ext.rs，不可删除） */
export const BUILTIN_LANGUAGES: LangOption[] = [
  { value: "rust", label: "Rust", builtin: true, highlight: true },
  { value: "go", label: "Go", builtin: true, highlight: true },
  { value: "python", label: "Python", builtin: true, highlight: true },
  { value: "javascript", label: "JavaScript", builtin: true, highlight: true },
  { value: "typescript", label: "TypeScript", builtin: true, highlight: true },
  { value: "vue", label: "Vue", builtin: true, highlight: true },
  { value: "java", label: "Java", builtin: true, highlight: true },
  { value: "kotlin", label: "Kotlin", builtin: true, highlight: true },
  { value: "swift", label: "Swift", builtin: true, highlight: true },
  { value: "c", label: "C", builtin: true, highlight: true },
  { value: "cpp", label: "C++", builtin: true, highlight: true },
  { value: "csharp", label: "C#", builtin: true, highlight: true },
  { value: "ruby", label: "Ruby", builtin: true, highlight: true },
  { value: "php", label: "PHP", builtin: true, highlight: true },
  { value: "shell", label: "Shell / Bash", builtin: true, highlight: true },
  { value: "sql", label: "SQL", builtin: true, highlight: true },
  { value: "html", label: "HTML", builtin: true, highlight: true },
  { value: "css", label: "CSS", builtin: true, highlight: true },
  { value: "scss", label: "SCSS / Sass", builtin: true, highlight: true },
  { value: "json", label: "JSON", builtin: true, highlight: true },
  { value: "yaml", label: "YAML", builtin: true, highlight: true },
  { value: "toml", label: "TOML", builtin: true, highlight: false },
  { value: "xml", label: "XML", builtin: true, highlight: true },
  { value: "markdown", label: "Markdown", builtin: true, highlight: true },
  { value: "lua", label: "Lua", builtin: true, highlight: true },
  { value: "r", label: "R", builtin: true, highlight: true },
  { value: "scala", label: "Scala", builtin: true, highlight: true },
  { value: "dart", label: "Dart", builtin: true, highlight: true },
  { value: "perl", label: "Perl", builtin: true, highlight: true },
  { value: "haskell", label: "Haskell", builtin: true, highlight: true },
  { value: "elixir", label: "Elixir", builtin: true, highlight: true },
  { value: "clojure", label: "Clojure", builtin: true, highlight: true },
  { value: "dockerfile", label: "Dockerfile", builtin: true, highlight: true },
  { value: "makefile", label: "Makefile", builtin: true, highlight: false },
  { value: "graphql", label: "GraphQL", builtin: true, highlight: true },
  { value: "proto", label: "Protocol Buffers", builtin: true, highlight: false },
  { value: "powershell", label: "PowerShell", builtin: true, highlight: true },
  { value: "ini", label: "INI / Conf", builtin: true, highlight: false },
  { value: "text", label: "纯文本", builtin: true, highlight: false },
];

/** localStorage key：自定义语言 */
const CUSTOM_LANG_KEY = "byte-link-custom-languages";

/** 读取自定义语言（localStorage 持久化） */
export function loadCustomLanguages(): LangOption[] {
  try {
    const raw = localStorage.getItem(CUSTOM_LANG_KEY);
    if (!raw) return [];
    const list = JSON.parse(raw) as LangOption[];
    return list.map((l) => ({ ...l, builtin: false }));
  } catch {
    return [];
  }
}

/** 保存自定义语言 */
export function saveCustomLanguages(list: LangOption[]) {
  localStorage.setItem(CUSTOM_LANG_KEY, JSON.stringify(list));
}

/** 全部语言（内置 + 自定义），去重 */
export function allLanguages(): LangOption[] {
  const custom = loadCustomLanguages();
  const builtinValues = new Set(BUILTIN_LANGUAGES.map((l) => l.value.toLowerCase()));
  const dedupedCustom = custom.filter((l) => !builtinValues.has(l.value.toLowerCase()));
  return [...BUILTIN_LANGUAGES, ...dedupedCustom];
}

/** a-auto-complete / a-select 的 options */
export const LANG_OPTIONS = BUILTIN_LANGUAGES.map((l) => ({ value: l.value, label: l.label }));

/** 按 value 查显示名 */
export function langLabel(value: string): string {
  const found = allLanguages().find((l) => l.value.toLowerCase() === value.toLowerCase());
  return found ? found.label : value;
}

/** 按 value 查是否支持高亮 */
export function langHighlight(value: string): boolean {
  const found = allLanguages().find((l) => l.value.toLowerCase() === value.toLowerCase());
  return found ? found.highlight : false;
}
