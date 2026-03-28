<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import gsap from "gsap";
import { motionDisabled } from "@/composables/useMotion";
/**
 * 统一空状态组件（全局复用）
 *
 * 设计语言（对齐全应用 Feather 线性图标体系）：
 * - 内置自绘线性图标（name: memo/trash/star/tag/folder/clip/search/history/inbox/layers），
 *   圆形浅底徽章承载；也兼容直接传 antd 图标组件（icon prop，旧用法不变）
 * - 标题（secondary/600）+ 引导文案（tertiary）说明「为什么空、下一步做什么」
 * - 可选动作按钮：actionText + @action（primary=true 时为渐变主 CTA，默认 ghost pill）
 *
 * Props:
 * - text: 主标题（兼容旧用必填）；desc: 引导文案（替代 hint，hint 仍兼容）
 * - name: 内置自绘图标键；icon: antd 图标组件（二选一）
 * - size: 'default'（面板级） | 'small'（图表/侧栏内嵌）；inline: 跟随流式布局
 * - actionText + primary: 动作按钮
 *
 * 用法：
 *   <EmptyState name="memo" text="还没有备忘录" desc="在上面记下第一笔" />
 *   <EmptyState name="tag" text="暂无标签" size="small" inline />
 *   <EmptyState name="search" text="无匹配结果" action-text="清空筛选" @action="clear" />
 */

/** 内置线性图标（Feather 24 网格，静态内置字符串，无用户输入） */
const ICON_PATHS: Record<string, string> = {
  memo: '<rect x="4" y="3" width="16" height="18" rx="3"/><line x1="8.5" y1="8" x2="15.5" y2="8"/><line x1="8.5" y1="12" x2="15.5" y2="12"/><line x1="8.5" y1="16" x2="12.5" y2="16"/>',
  trash: '<polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/>',
  star: '<polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>',
  tag: '<path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.83z"/><line x1="7" y1="7" x2="7.01" y2="7"/>',
  folder: '<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>',
  clip: '<path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/>',
  search: '<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>',
  history: '<path d="M3 3v5h5"/><path d="M3.05 13A9 9 0 1 0 6 5.3L3 8"/><polyline points="12 7 12 12 15 15"/>',
  inbox: '<polyline points="22 12 16 12 14 15 10 15 8 12 2 12"/><path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>',
  layers: '<polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/>',
  sparkles: '<path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"/><path d="M20 3v4"/><path d="M22 5h-4"/><path d="M4 17v2"/><path d="M5 18H3"/>',
};

const props = withDefaults(
  defineProps<{
    /** 内置自绘图标键（优先于 icon） */
    name?: string;
    /** 图标组件（antd 图标，兼容旧用法） */
    icon?: any;
    /** 主标题 */
    text: string;
    /** 引导文案：为什么空、下一步做什么 */
    desc?: string;
    /** 兼容旧参数：等价 desc */
    hint?: string;
    /** 尺寸：default=面板级；small=图表/内嵌紧凑 */
    size?: "default" | "small";
    /** 内嵌模式：跟随父容器流式布局（列表/表格内） */
    inline?: boolean;
    /** 动作按钮文案（不传则无按钮） */
    actionText?: string;
    /** 动作按钮为主 CTA（渐变 pill）；默认 ghost */
    primary?: boolean;
  }>(),
  {
    name: "",
    icon: null,
    desc: "",
    hint: "",
    size: "default",
    inline: false,
    actionText: "",
    primary: false,
  },
);

defineEmits<{ (e: "action"): void }>();

const svgInner = computed(() => (props.name && ICON_PATHS[props.name]) || "");

// 入场动画：徽章弹入 + 文案错峰淡入。只动子元素，不碰外层 transform
// （absolute 居中靠 translate(-50%,-50%)，动它会破坏居中）。
const rootRef = ref<HTMLElement>();
onMounted(() => {
  const el = rootRef.value;
  if (!el) return;
  if (motionDisabled()) return;
  const badge = el.querySelector<HTMLElement>(".empty-badge");
  if (badge) {
    gsap.fromTo(badge, { scale: 0.8, autoAlpha: 0 }, { scale: 1, autoAlpha: 1, clearProps: "all" });
  }
  const texts = el.querySelectorAll<HTMLElement>(".empty-title, .empty-desc, .empty-cta");
  if (texts.length) {
    gsap.fromTo(
      texts,
      { y: 6, autoAlpha: 0 },
      { y: 0, autoAlpha: 1, stagger: 0.06, delay: 0.05, clearProps: "all" },
    );
  }
});
</script>

<template>
  <div
    ref="rootRef"
    class="empty-state"
    :class="[size, { inline, absolute: !inline }]"
  >
    <div v-if="svgInner || icon" class="empty-badge">
      <svg
        v-if="svgInner"
        class="empty-svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.6"
        stroke-linecap="round"
        stroke-linejoin="round"
        v-html="svgInner"
      />
      <component :is="icon" v-else-if="icon" class="empty-antd-icon" />
    </div>
    <div class="empty-title">{{ text }}</div>
    <div v-if="desc || hint" class="empty-desc">{{ desc || hint }}</div>
    <button v-if="actionText" class="empty-cta" :class="{ primary }" @click="$emit('action')">
      {{ actionText }}
    </button>
  </div>
</template>

<style scoped>
/*
 * 定位策略：作为 panel（flex column）的直接子项，用 flex:1 占满 header 之外的
 * 剩余空间，内部 flex 居中。absolute 模式相对 panel 中心定位（兼容旧布局）。
 */

/* ============ 通用结构 ============ */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  text-align: center;
}

/* absolute（默认）：面板级空状态，相对 panel 中心居中 */
.empty-state.absolute {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  pointer-events: none;
}
/* inline：内嵌，跟随父容器流式布局（列表/图表内） */
.empty-state.inline {
  display: flex;
}

/* ============ 徽章（圆形浅底 + 线性图标） ============ */
.empty-badge {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--text-primary) 4%, transparent);
  color: var(--text-tertiary);
}
.empty-svg {
  width: 22px;
  height: 22px;
}
.empty-antd-icon {
  font-size: 22px;
}

/* ============ 文案 ============ */
.empty-title {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-secondary);
}
.empty-desc {
  font-size: 11px;
  line-height: 1.7;
  color: var(--text-tertiary);
  max-width: 280px;
}

/* ============ 动作按钮 ============ */
.empty-cta {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  padding: 0 14px;
  margin-top: 4px;
  font-size: 11.5px;
  font-family: inherit;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid var(--border-light);
  border-radius: 999px;
  cursor: pointer;
  transition: all var(--transition-fast);
  pointer-events: auto;
}
.empty-cta:hover {
  color: var(--text-primary);
  border-color: var(--text-tertiary);
}
.empty-cta.primary {
  color: var(--hero-cta-text, #fff);
  background: linear-gradient(135deg, var(--color-primary), color-mix(in srgb, var(--color-primary) 82%, #000));
  border: none;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--color-primary) 24%, transparent);
}
.empty-cta.primary:hover {
  filter: brightness(1.08);
}

/* ============ 尺寸：small（图表/侧栏内嵌，紧凑） ============ */
.empty-state.small {
  gap: 6px;
}
.empty-state.small .empty-badge {
  width: 34px;
  height: 34px;
}
.empty-state.small .empty-svg,
.empty-state.small .empty-antd-icon {
  width: 17px;
  height: 17px;
  font-size: 17px;
}
.empty-state.small .empty-title {
  font-size: 12px;
}
.empty-state.small .empty-desc {
  font-size: 10.5px;
  max-width: 220px;
}
.empty-state.small .empty-cta {
  height: 24px;
  padding: 0 12px;
  font-size: 11px;
}
</style>
