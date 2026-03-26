<script setup lang="ts">
/**
 * byte-memos Logo（备忘录笔记 · 透明背景 · 动态着色）
 *
 * 圆角笔记卡片 + 三条文字行镂空：直接对应「备忘录管理」，
 * 轮廓简洁，小尺寸下仍有辨识度。
 *
 * 着色：SVG 用 fill="currentColor"，颜色由 CSS color 控制——
 *   - 不传 color：继承父元素文字色（浅主题自动深、深主题自动浅）
 *   - 传 color：用指定色（如主色蓝 var(--color-primary)）
 *
 * 本体透明无背景方块，因此不会有黑边/底色问题，任意背景上都能融入。
 */
import { useId } from "vue";

withDefaults(defineProps<{
  /** 图标尺寸（px），正方形 */
  size?: number;
  /** 颜色（CSS color 值），不传则继承父元素 currentColor */
  color?: string;
}>(), {
  size: 26,
  color: "currentColor",
});

// 每个实例唯一 mask id，避免多实例 mask 引用错乱
const maskId = `logo-mask-${useId()}`;
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 64 64"
    :style="{ color }"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-label="byte-memos"
    role="img"
  >
    <defs>
      <!-- 文字行镂空 mask：白=保留，黑=挖空（透出背景） -->
      <mask :id="maskId">
        <rect width="64" height="64" fill="white" />
        <rect x="21" y="20" width="22" height="4" rx="2" fill="black" />
        <rect x="21" y="30" width="22" height="4" rx="2" fill="black" />
        <rect x="21" y="40" width="13" height="4" rx="2" fill="black" />
      </mask>
    </defs>
    <!-- 备忘录卡片（圆角矩形 + 文字行镂空） -->
    <rect
      x="12"
      y="8"
      width="40"
      height="48"
      rx="8"
      fill="currentColor"
      :mask="`url(#${maskId})`"
    />
  </svg>
</template>
