<script setup lang="ts">
/**
 * 时间轴视图（byte-memos 原创设计，原版 memos 无此形态）
 *
 * - 按本地创建日期分组的纵向轨道：sticky 日期头（相对日 今天/昨天 + 星期 + 当日条数）
 * - 每条备忘录挂轨：左侧圆点（分类色联动）+ 时间 + 标题/摘要/标签，悬浮显操作
 * - 置顶条目带图钉标；轨道线用 border + 负缩进实现，纯 CSS 无依赖
 */
import { computed } from "vue";
import {
  PushpinFilled,
  PushpinOutlined,
  EditOutlined,
  DeleteOutlined,
} from "@ant-design/icons-vue";
import type { Memo } from "@/api/types";

const props = defineProps<{
  memos: Memo[];
  /** 分类名 → 主题色（时间轴圆点联动） */
  catColorMap: Map<string, string>;
}>();
const emit = defineEmits<{
  (e: "open", m: Memo): void;
  (e: "pin", m: Memo): void;
  (e: "del", m: Memo): void;
}>();

interface Entry {
  memo: Memo;
  time: string; // HH:mm
}
interface DayGroup {
  key: string; // YYYY-MM-DD
  label: string; // 今天 / 昨天 / M月D日 周X（跨年带年份）
  count: number;
  isToday: boolean;
  entries: Entry[];
}

const WEEK = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"];

function localParts(iso: string): { key: string; time: string } {
  const d = new Date(iso);
  const p = (n: number) => String(n).padStart(2, "0");
  return {
    key: `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`,
    time: `${p(d.getHours())}:${p(d.getMinutes())}`,
  };
}

function dayLabel(key: string): string {
  const [y, m, dd] = key.split("-").map(Number);
  const d = new Date(y, m - 1, dd);
  const today = new Date();
  const tKey = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
  const yesterday = new Date(today.getFullYear(), today.getMonth(), today.getDate() - 1);
  const yKey = `${yesterday.getFullYear()}-${String(yesterday.getMonth() + 1).padStart(2, "0")}-${String(yesterday.getDate()).padStart(2, "0")}`;
  const base = `${m}月${dd}日 ${WEEK[d.getDay()]}`;
  if (key === tKey) return `今天 · ${base}`;
  if (key === yKey) return `昨天 · ${base}`;
  const yearPrefix = y === today.getFullYear() ? "" : `${y}年`;
  return `${yearPrefix}${base}`;
}

const groups = computed<DayGroup[]>(() => {
  const map = new Map<string, Entry[]>();
  for (const m of props.memos) {
    const { key, time } = localParts(m.created_at);
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push({ memo: m, time });
  }
  const today = new Date();
  const tKey = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
  return [...map.entries()]
    .sort((a, b) => (a[0] < b[0] ? 1 : -1))
    .map(([key, entries]) => ({
      key,
      label: dayLabel(key),
      count: entries.length,
      isToday: key === tKey,
      entries,
    }));
});

function dotColor(m: Memo): string {
  return props.catColorMap.get(m.category ?? "") || "var(--color-primary)";
}
</script>

<template>
  <div class="memo-timeline">
    <div v-for="g in groups" :key="g.key" class="tl-group" :class="{ today: g.isToday }">
      <div class="tl-day-head">
        <span class="tl-day-label">{{ g.label }}</span>
        <span class="tl-day-count">{{ g.count }} 条</span>
      </div>

      <div class="tl-entries">
        <div
          v-for="entry in g.entries"
          :key="entry.memo.short_id"
          class="tl-entry"
          @click="emit('open', entry.memo)"
        >
          <span class="tl-dot" :class="{ pin: entry.memo.pinned }" :style="{ background: dotColor(entry.memo) }" />
          <span class="tl-time">{{ entry.time }}</span>

          <div class="tl-body">
            <div class="tl-title-line">
              <PushpinFilled v-if="entry.memo.pinned" class="tl-pin" />
              <span class="tl-title">{{ entry.memo.title || "无标题" }}</span>
              <a-tag v-if="entry.memo.category" :color="catColorMap.get(entry.memo.category) || 'geekblue'" class="tl-tag">
                {{ entry.memo.category }}
              </a-tag>
              <a-tag v-for="t in entry.memo.tags.slice(0, 3)" :key="t" class="tl-tag">#{{ t }}</a-tag>
            </div>
            <div class="tl-excerpt">{{ entry.memo.content.replace(/[#>*_~`-]+/g, " ").replace(/\s+/g, " ").trim().slice(0, 90) }}</div>
          </div>

          <div class="tl-ops">
            <button
              class="tl-op"
              :class="{ on: entry.memo.pinned }"
              :title="entry.memo.pinned ? '取消置顶' : '置顶'"
              @click.stop="emit('pin', entry.memo)"
            >
              <PushpinFilled v-if="entry.memo.pinned" /><PushpinOutlined v-else />
            </button>
            <button class="tl-op" title="编辑" @click.stop="emit('open', entry.memo)"><EditOutlined /></button>
            <button class="tl-op danger" title="删除" @click.stop="emit('del', entry.memo)"><DeleteOutlined /></button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.memo-timeline {
  width: 100%; /* 列宽由父级 .flow 统一约束（780px 阅读列） */
}

.tl-group {
  margin-bottom: 6px;
}

.tl-day-head {
  position: sticky;
  top: 0;
  z-index: 2;
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 10px 2px 7px;
  background: linear-gradient(to bottom, var(--bg-surface) 78%, transparent);
}
.tl-day-label {
  font-family: "Noto Serif SC", "Source Han Serif SC", "STZhongsong", serif;
  font-size: 13.5px;
  font-weight: 600;
  letter-spacing: 0.02em;
  color: var(--text-primary);
}
.tl-group.today .tl-day-label {
  color: var(--color-primary);
}
.tl-day-count {
  font-size: 10px;
  color: var(--text-tertiary);
  background: color-mix(in srgb, var(--text-primary) 5%, transparent);
  border-radius: 999px;
  padding: 1px 7px;
  font-family: "JetBrains Mono", monospace;
}

/* 轨道：左线 + 缩进 */
.tl-entries {
  position: relative;
  margin-left: 14px;
  padding-left: 18px;
  border-left: 2px solid var(--border-light);
}
.tl-group.today .tl-entries {
  border-left-color: color-mix(in srgb, var(--color-primary) 45%, transparent);
}

.tl-entry {
  position: relative;
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 10px;
  margin-bottom: 2px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background var(--transition-fast);
}
.tl-entry:hover {
  background: color-mix(in srgb, var(--color-primary) 4%, transparent);
}

.tl-dot {
  position: absolute;
  left: -23px; /* -(18 padding + 2 line/2 + dot/2 ≈) 对齐轨道线 */
  top: 16px;
  width: 9px;
  height: 9px;
  border-radius: 50%;
  box-shadow: 0 0 0 3px var(--bg-surface);
}
.tl-dot.pin {
  box-shadow:
    0 0 0 3px var(--bg-surface),
    0 0 0 4.5px color-mix(in srgb, var(--color-primary) 35%, transparent);
}

.tl-time {
  flex-shrink: 0;
  width: 38px;
  margin-top: 2px;
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
}

.tl-body {
  flex: 1;
  min-width: 0;
}
.tl-title-line {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.tl-pin {
  font-size: 11px;
  color: var(--color-primary);
}
.tl-title {
  font-size: 13.5px;
  font-weight: 600;
  letter-spacing: -0.2px;
  color: var(--text-primary);
}
.tl-tag {
  margin-right: 0;
  font-size: 10.5px;
  line-height: 17px;
  padding: 0 6px;
}
.tl-excerpt {
  margin-top: 3px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tl-ops {
  display: flex;
  gap: 2px;
  opacity: 0;
  transform: translateX(4px);
  transition: opacity var(--transition-fast), transform var(--transition-fast);
}
.tl-entry:hover .tl-ops {
  opacity: 1;
  transform: translateX(0);
}
.tl-op {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  font-size: 13px;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: 999px;
  cursor: pointer;
  transition: color var(--transition-fast), background var(--transition-fast);
}
.tl-op:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 7%, transparent);
}
.tl-op.on {
  color: var(--color-primary);
}
.tl-op.danger:hover {
  color: #ef4444;
}
</style>
