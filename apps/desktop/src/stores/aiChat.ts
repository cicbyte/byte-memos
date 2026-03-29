/**
 * AI 侧边栏对话状态：多会话 + 历史记录（localStorage 持久化）
 *
 * - sessions：会话数组（标题/时间/消息），当前会话由 currentId 指定
 * - 新对话 = 另起新会话（旧会话保留在历史列表，上限 30 个）
 * - 兼容迁移：旧版单会话键 bm-ai-chat 自动转为首个会话
 * - send：流式发送；助手消息以占位形式进入列表，增量实时写入
 * - stop：取消（取消后空回复占位自动移除）
 */
import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { message } from "ant-design-vue";
import { streamChat, isCancelledError, type StreamHandle, type AiStreamChunk } from "@/api/ai";
import { useAiProviderStore } from "@/stores/aiProvider";

/** 工具调用步骤（一轮助手回复内的执行记录，ZCode ToolCallBlock 模式） */
export interface ToolStep {
  /** 工具名（如 search_memos） */
  name: string;
  /** 原始参数 JSON（展示层解析摘要） */
  args: string;
  /** running=执行中（spinner）；done=完成（对勾） */
  status: "running" | "done";
  /** 执行结果 JSON 文本（工具结束时回传；展示层解析成败） */
  result?: string;
  /** 发起时间戳（ms，用于步骤耗时展示；旧数据为 0） */
  t: number;
}

/** 对话消息（助手轮可携带工具调用步骤） */
export interface ChatTurn {
  role: "user" | "assistant";
  content: string;
  steps?: ToolStep[];
}

/** 会话（含消息；store 内部使用） */
export interface ChatSession {
  id: string;
  title: string;
  /** 最后活跃时间戳（ms） */
  updatedAt: number;
  messages: ChatTurn[];
}

/** 会话摘要（历史列表展示用） */
export interface ChatSessionMeta {
  id: string;
  title: string;
  updatedAt: number;
  /** 消息条数 */
  count: number;
}

const LEGACY_KEY = "bm-ai-chat";
const SESSIONS_KEY = "bm-ai-chat-sessions";
const THINKING_KEY = "bm-ai-thinking";
const MAX_SESSIONS = 30;
const MAX_MESSAGES = 100;

/** 思考等级（off=关 / medium=适中默认 / high=深入），localStorage 持久化 */
function loadThinking(): string {
  const v = localStorage.getItem(THINKING_KEY);
  return v === "off" || v === "high" ? v : "medium";
}

/** 解析消息数组（严格校验：坏数据丢弃；兼容旧 tools 字符串数组 → steps） */
function parseTurns(arr: unknown[]): ChatTurn[] {
  const out: ChatTurn[] = [];
  for (const t of arr) {
    if (
      t === null || typeof t !== "object" ||
      ((t as ChatTurn).role !== "user" && (t as ChatTurn).role !== "assistant") ||
      typeof (t as ChatTurn).content !== "string"
    ) {
      continue;
    }
    const rec = t as Record<string, unknown>;
    const turn: ChatTurn = { role: rec.role as ChatTurn["role"], content: rec.content as string };
    const src = Array.isArray(rec.steps)
      ? (rec.steps as unknown[])
      : Array.isArray(rec.tools)
        ? (rec.tools as unknown[])
        : [];
    const steps: ToolStep[] = [];
    for (const s of src) {
      if (s !== null && typeof s === "object") {
        const o = s as { name?: unknown; args?: unknown; status?: unknown; result?: unknown; t?: unknown };
        if (typeof o.name === "string") {
          steps.push({
            name: o.name,
            args: typeof o.args === "string" ? o.args : "",
            status: o.status === "running" ? "running" : "done",
            result: typeof o.result === "string" ? o.result : undefined,
            t: typeof o.t === "number" ? o.t : 0,
          });
        }
      } else if (typeof s === "string") {
        steps.push({ name: s, args: "", status: "done", t: 0 });
      }
    }
    if (steps.length) turn.steps = steps;
    out.push(turn);
  }
  return out;
}

/** 解析单条消息数组（localStorage JSON） */
function parseTurnArray(raw: unknown): ChatTurn[] {
  return Array.isArray(raw) ? parseTurns(raw) : [];
}

/** 由首个用户消息派生会话标题 */
function deriveTitle(messages: ChatTurn[]): string {
  const first = messages.find((m) => m.role === "user");
  if (!first) return "新对话";
  const t = first.content.trim().replace(/\s+/g, " ");
  return t.length <= 20 ? t : `${t.slice(0, 20)}…`;
}

function newSession(messages: ChatTurn[] = []): ChatSession {
  return {
    id: crypto.randomUUID(),
    title: deriveTitle(messages),
    updatedAt: Date.now(),
    messages,
  };
}

/** 载入会话存储；无有效数据时迁移旧版单会话键或建空会话 */
function loadSessions(): ChatSession[] {
  try {
    const raw = localStorage.getItem(SESSIONS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as unknown;
      if (parsed && typeof parsed === "object" && Array.isArray((parsed as { sessions?: unknown }).sessions)) {
        const sessions = ((parsed as { sessions: unknown[] }).sessions)
          .filter((s): s is Record<string, unknown> => s !== null && typeof s === "object" && typeof (s as { id?: unknown }).id === "string")
          .map((s) => ({
            id: s.id as string,
            title: typeof s.title === "string" && s.title ? s.title : "新对话",
            updatedAt: typeof s.updatedAt === "number" ? s.updatedAt : 0,
            messages: parseTurnArray(s.messages),
          }));
        if (sessions.length) return sessions.slice(-MAX_SESSIONS);
      }
    }
  } catch {
    /* 损坏则走迁移/新建 */
  }
  // 迁移旧版单会话
  try {
    const legacy = localStorage.getItem(LEGACY_KEY);
    if (legacy) {
      const turns = parseTurnArray(JSON.parse(legacy) as unknown);
      if (turns.length) {
        const s = newSession(turns);
        try {
          localStorage.removeItem(LEGACY_KEY);
        } catch {
          /* 忽略 */
        }
        return [s];
      }
    }
  } catch {
    /* 忽略 */
  }
  return [newSession()];
}

export const useAiChatStore = defineStore("aiChat", () => {
  const aiStore = useAiProviderStore();
  const open = ref(false);
  const sessions = ref<ChatSession[]>(loadSessions());
  const currentId = ref(sessions.value[sessions.value.length - 1].id);
  const running = ref(false);
  const thinking = ref(loadThinking());
  let handle: StreamHandle | null = null;

  /** 当前会话消息（computed 代理进 sessions；模板/逻辑全部照旧用 chat.messages） */
  const messages = computed<ChatTurn[]>({
    get: () => sessions.value.find((s) => s.id === currentId.value)?.messages ?? [],
    set: (v) => {
      const s = sessions.value.find((x) => x.id === currentId.value);
      if (s) s.messages = v;
    },
  });

  /** 历史列表（最近活跃在前） */
  const sessionList = computed<ChatSessionMeta[]>(() =>
    [...sessions.value]
      .sort((a, b) => b.updatedAt - a.updatedAt)
      .map(({ id, title, updatedAt, messages }) => ({ id, title, updatedAt, count: messages.length })),
  );

  function persist() {
    try {
      // 超限裁剪：保留最近活跃的 MAX_SESSIONS 个（当前会话必须在内）
      let list = [...sessions.value].sort((a, b) => b.updatedAt - a.updatedAt).slice(0, MAX_SESSIONS);
      if (!list.some((s) => s.id === currentId.value)) list = [sessions.value.find((s) => s.id === currentId.value)!, ...list].slice(0, MAX_SESSIONS);
      sessions.value = list;
      localStorage.setItem(SESSIONS_KEY, JSON.stringify({ currentId: currentId.value, sessions: list }));
    } catch {
      /* 存储满则忽略 */
    }
  }

  /** 设置思考等级（off/medium/high）并持久化 */
  function setThinking(v: string) {
    thinking.value = v;
    try {
      localStorage.setItem(THINKING_KEY, v);
    } catch {
      /* 忽略 */
    }
  }

  function toggle(v?: boolean) {
    open.value = v ?? !open.value;
    if (open.value) void aiStore.load();
  }

  function touch() {
    const s = sessions.value.find((x) => x.id === currentId.value);
    if (s) s.updatedAt = Date.now();
  }

  /** 新对话：另起新会话（旧会话保留在历史） */
  function newChat() {
    stop();
    const cur = sessions.value.find((s) => s.id === currentId.value);
    // 当前已是空会话则复用
    if (cur && cur.messages.length === 0) return;
    const s = newSession();
    sessions.value.push(s);
    currentId.value = s.id;
    persist();
  }

  /** 切换到历史会话 */
  function switchSession(id: string) {
    if (id === currentId.value) return;
    stop();
    currentId.value = id;
    persist();
  }

  /** 删除会话（删的是当前会话则切到最近的，一个不剩时新建空会话） */
  function deleteSession(id: string) {
    sessions.value = sessions.value.filter((s) => s.id !== id);
    if (id === currentId.value) {
      const rest = [...sessions.value].sort((a, b) => b.updatedAt - a.updatedAt);
      if (rest.length) {
        currentId.value = rest[0].id;
      } else {
        const s = newSession();
        sessions.value.push(s);
        currentId.value = s.id;
      }
    }
    persist();
  }

  function stop() {
    handle?.cancel();
    handle = null;
    running.value = false;
    // 移除末尾空的助手占位
    const last = messages.value[messages.value.length - 1];
    if (last && last.role === "assistant" && !last.content.trim()) {
      messages.value.pop();
    }
    persist();
  }

  async function send(text: string) {
    const t = text.trim();
    if (!t || running.value) return;
    if (!(await aiStore.ensureReady())) return;
    // 默认模型必须由用户显式选择（内置供应商不再预置）
    const p = aiStore.enabledProvider();
    if (!p?.model) {
      message.warning("当前供应商还未选择模型：点击底部模型名即可选择");
      return;
    }

    const history: ChatTurn[] = [...messages.value, { role: "user", content: t }];
    const placeholder: ChatTurn = { role: "assistant", content: "" };
    messages.value = [...history, placeholder].slice(-MAX_MESSAGES);
    // 标题：仍为默认时取首条用户消息
    const cur = sessions.value.find((s) => s.id === currentId.value);
    if (cur && (!cur.title || cur.title === "新对话")) cur.title = deriveTitle(messages.value);
    touch();
    running.value = true;

    const write = (fn: (a: ChatTurn) => void) => {
      const arr = messages.value;
      const last = arr[arr.length - 1];
      if (last && last.role === "assistant") {
        fn(last);
        messages.value = [...arr];
      }
    };

    handle = streamChat(history, thinking.value, (chunk: AiStreamChunk) => {
      if (chunk.type === "token") {
        write((a) => {
          a.content += chunk.content;
          // 正文开始流出 = 前序工具轮已执行完，收束 running 状态
          if (a.steps) a.steps.forEach((s) => { if (s.status === "running") s.status = "done"; });
        });
      } else if (chunk.type === "tool") {
        write((a) => {
          a.steps = [...(a.steps ?? []), { name: chunk.name, args: chunk.args, status: "running", t: Date.now() }];
        });
      } else if (chunk.type === "tool_result") {
        write((a) => {
          if (a.steps?.length) {
            // 同轮同名工具按调用顺序落位：找第一个无结果的同名步骤
            const idx = a.steps.findIndex((s) => s.name === chunk.name && s.result === undefined);
            const target = a.steps[idx >= 0 ? idx : a.steps.length - 1];
            // 截断超长结果（防 localStorage 膨胀；详情展示足够）
            target.result = chunk.result.length > 2000 ? `${chunk.result.slice(0, 2000)}…` : chunk.result;
            target.status = "done";
          }
        });
      } else if (chunk.type === "error") {
        write((a) => {
          a.content += `\n\n> ⚠ ${chunk.message}`;
          if (a.steps) a.steps.forEach((s) => { if (s.status === "running") s.status = "done"; });
        });
      }
    });

    handle.done
      .then(() => {
        running.value = false;
        handle = null;
        // 收尾兜底：全部步骤置 done
        const last = messages.value[messages.value.length - 1];
        if (last && last.role === "assistant" && last.steps?.some((s) => s.status === "running")) {
          last.steps.forEach((s) => { s.status = "done"; });
          messages.value = [...messages.value];
        }
        touch();
        persist();
      })
      .catch((e) => {
        running.value = false;
        handle = null;
        if (!isCancelledError(e)) {
          message.error(e instanceof Error ? e.message : String(e));
        }
        // 取消/失败：移除空占位
        const last = messages.value[messages.value.length - 1];
        if (last && last.role === "assistant" && !last.content.trim()) {
          messages.value.pop();
        }
        touch();
        persist();
      });
  }

  return {
    open, messages, running, thinking, currentId, sessionList,
    toggle, send, stop, newChat, setThinking, switchSession, deleteSession,
  };
});
