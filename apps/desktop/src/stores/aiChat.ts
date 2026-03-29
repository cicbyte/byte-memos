/**
 * AI 侧边栏对话状态：全局唯一会话（localStorage 持久化，上限 100 条）
 *
 * - open：侧栏开关（Navbar 按钮 / AppLayout 挂载 AiSidebar）
 * - send：流式发送；助手消息以占位形式进入列表，增量实时写入
 * - stop：取消（取消后空回复占位自动移除）
 */
import { defineStore } from "pinia";
import { ref } from "vue";
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

const STORAGE_KEY = "bm-ai-chat";
const MAX_MESSAGES = 100;

function loadTurns(): ChatTurn[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const arr = JSON.parse(raw) as unknown;
    if (!Array.isArray(arr)) return [];
    return arr
      .filter(
        (t): t is Record<string, unknown> =>
          t !== null && typeof t === "object" &&
          ((t as ChatTurn).role === "user" || (t as ChatTurn).role === "assistant") &&
          typeof (t as ChatTurn).content === "string",
      )
      .map((t) => {
        const turn: ChatTurn = {
          role: t.role as ChatTurn["role"],
          content: t.content as string,
        };
        // 兼容旧数据：steps 对象数组，或更早的 tools 字符串数组
        const src = Array.isArray(t.steps)
          ? (t.steps as unknown[])
          : Array.isArray(t.tools)
            ? (t.tools as unknown[])
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
        return turn;
      });
  } catch {
    return [];
  }
}

export const useAiChatStore = defineStore("aiChat", () => {
  const aiStore = useAiProviderStore();
  const open = ref(false);
  const messages = ref<ChatTurn[]>(loadTurns());
  const running = ref(false);
  let handle: StreamHandle | null = null;

  function persist() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(messages.value.slice(-MAX_MESSAGES)));
    } catch {
      /* 存储满则忽略 */
    }
  }

  function toggle(v?: boolean) {
    open.value = v ?? !open.value;
    if (open.value) void aiStore.load();
  }

  function newChat() {
    stop();
    messages.value = [];
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
      message.warning("当前供应商还未选择模型：点击顶部模型名即可选择");
      return;
    }

    const history: ChatTurn[] = [...messages.value, { role: "user", content: t }];
    const placeholder: ChatTurn = { role: "assistant", content: "" };
    messages.value = [...history, placeholder].slice(-MAX_MESSAGES);
    running.value = true;

    const write = (fn: (a: ChatTurn) => void) => {
      const arr = messages.value;
      const last = arr[arr.length - 1];
      if (last && last.role === "assistant") {
        fn(last);
        messages.value = [...arr];
      }
    };

    handle = streamChat(history, (chunk: AiStreamChunk) => {
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
        persist();
      });
  }

  return { open, messages, running, toggle, send, stop, newChat };
});
