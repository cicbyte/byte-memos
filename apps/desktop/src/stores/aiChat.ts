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

/** 对话消息（助手轮可携带工具调用记录） */
export interface ChatTurn {
  role: "user" | "assistant";
  content: string;
  /** 本轮助手用过的内置工具（按调用顺序，如 search_memos） */
  tools?: string[];
}

const STORAGE_KEY = "bm-ai-chat";
const MAX_MESSAGES = 100;

function loadTurns(): ChatTurn[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const arr = JSON.parse(raw) as unknown;
    if (!Array.isArray(arr)) return [];
    return arr.filter(
      (t): t is ChatTurn =>
        t !== null && typeof t === "object" &&
        ((t as ChatTurn).role === "user" || (t as ChatTurn).role === "assistant") &&
        typeof (t as ChatTurn).content === "string",
    );
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
        write((a) => (a.content += chunk.content));
      } else if (chunk.type === "tool") {
        write((a) => {
          a.tools = [...(a.tools ?? []), chunk.name];
        });
      } else if (chunk.type === "error") {
        write((a) => (a.content += `\n\n> ⚠ ${chunk.message}`));
      }
    });

    handle.done
      .then(() => {
        running.value = false;
        handle = null;
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
