/**
 * AI 供应商全局状态：唯一数据源 + 就绪守卫
 *
 * ensureReady()：任何 AI 调用前检查有无启用中的供应商，
 * 没有则提示并跳转 /settings（沿袭 byte-writer useAiProviderStore 模式）。
 */
import { defineStore } from "pinia";
import { ref } from "vue";
import { message } from "ant-design-vue";
import { api } from "@/api";
import type { AiProviderItem } from "@/api/ai";
import router from "@/router";

export const useAiProviderStore = defineStore("aiProvider", () => {
  const providers = ref<AiProviderItem[]>([]);
  const loaded = ref(false);

  async function load(force = false): Promise<AiProviderItem[]> {
    if (loaded.value && !force) return providers.value;
    try {
      providers.value = await api.list();
      loaded.value = true;
    } catch {
      providers.value = [];
    }
    return providers.value;
  }

  function enabledProvider(): AiProviderItem | undefined {
    return providers.value.find((p) => p.is_enabled);
  }

  /** 切换供应商的默认模型（显式用户动作；api_key 传空 = 保留原值） */
  async function selectModel(id: number, model: string): Promise<boolean> {
    const p = providers.value.find((x) => x.id === id);
    if (!p) return false;
    try {
      await api.update({
        id,
        name: p.name,
        base_url: p.base_url,
        api_key: "",
        model,
        max_tokens: p.max_tokens,
        temperature: p.temperature,
        is_enabled: p.is_enabled,
      });
      p.model = model;
      return true;
    } catch {
      message.error("模型切换失败");
      return false;
    }
  }

  /** AI 调用前守卫：返回是否就绪（未配置则提示并跳设置页） */
  async function ensureReady(): Promise<boolean> {
    const list = await load();
    if (list.some((p) => p.is_enabled)) return true;
    message.warning("尚未配置 AI 供应商，请先在设置中添加并启用");
    void router.push("/settings");
    return false;
  }

  return { providers, loaded, load, enabledProvider, selectModel, ensureReady };
});
