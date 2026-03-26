/**
 * 首页 hero 个性化（工具软件个性化取向，纯前端）
 *
 * 标语（主句/高亮句/副文案）与 CTA 动作可由用户自定义，localStorage 持久化。
 * 壁纸/主题等视觉个性化见 theme/wallpaper store；本 store 只管 hero 文案与动作。
 */
import { defineStore } from "pinia";
import { ref, watch } from "vue";

/** CTA 动作类型：新建备忘录 / 随手看一条（随机打开） / 打开回收站 */
export type HeroCtaAction = "new" | "random" | "trash";

export interface HeroConfig {
  /** 标题主句（hero-title 前段） */
  titleMain: string;
  /** 标题高亮句（hero-hl，主色 + 主题色底线） */
  titleHl: string;
  /** 副文案 */
  subtitle: string;
  /** CTA 动作 */
  ctaAction: HeroCtaAction;
}

export const HERO_DEFAULTS: HeroConfig = {
  titleMain: "念头随手记",
  titleHl: "，条条可寻回",
  subtitle: "本地优先的备忘录 —— Markdown 记录、全文检索、版本可回滚，数据全部属于你。",
  ctaAction: "new",
};

const STORAGE_KEY = "byte-memos-hero";

function load(): HeroConfig {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...HERO_DEFAULTS };
    const parsed = JSON.parse(raw) as Partial<HeroConfig>;
    return {
      titleMain: parsed.titleMain?.trim() || HERO_DEFAULTS.titleMain,
      titleHl: parsed.titleHl ?? HERO_DEFAULTS.titleHl,
      subtitle: parsed.subtitle ?? HERO_DEFAULTS.subtitle,
      ctaAction:
        parsed.ctaAction === "random" || parsed.ctaAction === "trash" ? parsed.ctaAction : "new",
    };
  } catch {
    return { ...HERO_DEFAULTS };
  }
}

export const useHeroStore = defineStore("hero", () => {
  const config = ref<HeroConfig>(load());

  function update(patch: Partial<HeroConfig>) {
    config.value = { ...config.value, ...patch };
  }
  function reset() {
    config.value = { ...HERO_DEFAULTS };
  }

  watch(config, (v) => localStorage.setItem(STORAGE_KEY, JSON.stringify(v)), { deep: true });

  return { config, update, reset };
});
