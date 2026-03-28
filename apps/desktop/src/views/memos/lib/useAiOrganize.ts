/**
 * 「AI 整理」composable：一次调用产出 标题 / 标签 / 分类 建议并解析
 *
 * 供快速输入 composer 与完整编辑弹框共用：
 * - ensureReady 守卫（未配置供应商 → 提示 + 跳设置）
 * - 流式调用 action=organize（lang 携带候选分类，与 core prompts.rs 约定一致）
 * - organizing 状态驱动按钮 loading；错误 toast 后返回 null
 */
import { ref } from "vue";
import { message } from "ant-design-vue";
import { streamAi, isCancelledError } from "@/api/ai";
import { useAiProviderStore } from "@/stores/aiProvider";
import { parseOrganize, type AiOrganize } from "../aiActions";

export function useAiOrganize() {
  const aiStore = useAiProviderStore();
  const organizing = ref(false);

  async function organize(
    content: string,
    title = "",
    categoryNames: string[] = [],
  ): Promise<AiOrganize | null> {
    const text = content.trim();
    if (!text || organizing.value) return null;
    if (!(await aiStore.ensureReady())) return null;
    organizing.value = true;
    try {
      const full = await streamAi(
        { action: "organize", content: text, title, lang: categoryNames.join(",") },
        () => {},
      ).done;
      const r = parseOrganize(full);
      if (!r.title && !r.tags.length && !r.category) {
        message.warning("AI 未给出整理结果，请重试");
        return null;
      }
      return r;
    } catch (e) {
      if (!isCancelledError(e)) {
        message.error(e instanceof Error ? e.message : String(e));
      }
      return null;
    } finally {
      organizing.value = false;
    }
  }

  return { organizing, organize };
}

/** 合并去重标签（AI 建议 + 已有） */
export function mergeTags(base: string[], extra: string[]): string[] {
  return [...new Set([...base, ...extra])];
}
