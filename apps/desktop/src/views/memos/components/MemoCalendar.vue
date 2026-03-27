<script setup lang="ts">
/**
 * 月历热力图（参考 usememos/memos StatisticsView + ActivityCalendar，Vue 重写）
 *
 * - 7 列 × 6 行日期矩阵（周一首），按 当日条数/月内峰值 四档色阶染色（主题色 color-mix 联动）
 * - 点日期 → emit 选中（YYYY-MM-DD）；点月份标题 → 年视图（12 个迷你月历，跨年跳转）
 * - 无第三方日历依赖：纯原生 Date 计算
 */
import { computed, ref, watch } from "vue";
import { LeftOutlined, RightOutlined } from "@ant-design/icons-vue";

const props = defineProps<{
  /** 日期 → 当日条数（YYYY-MM-DD 本地日期，来自 memo_daily_counts） */
  counts: Record<string, number>;
  /** 当前选中日期（null = 未筛选） */
  selected: string | null;
}>();
const emit = defineEmits<{ (e: "select", date: string | null): void }>();

const today = new Date();
const todayKey = fmtKey(today);

const viewYear = ref(today.getFullYear());
const viewMonth = ref(today.getMonth() + 1); // 1-12
const yearView = ref(false);

watch(
  () => props.selected,
  (sel) => {
    if (!sel) return;
    const [y, m] = sel.split("-").map(Number);
    if (y && m) {
      viewYear.value = y;
      viewMonth.value = m;
    }
  },
);

function fmtKey(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

function shiftMonth(delta: number) {
  const d = new Date(viewYear.value, viewMonth.value - 1 + delta, 1);
  viewYear.value = d.getFullYear();
  viewMonth.value = d.getMonth() + 1;
}

function goToday() {
  viewYear.value = today.getFullYear();
  viewMonth.value = today.getMonth() + 1;
  emit("select", todayKey);
}

/** 月内峰值（色阶分母；空月为 0） */
const monthMax = computed(() => {
  let max = 0;
  for (const row of matrix.value) {
    for (const cell of row) {
      if (cell.key) {
        const c = props.counts[cell.key] || 0;
        if (c > max) max = c;
      }
    }
  }
  return max;
});

interface Cell {
  day: number;
  key: string | null; // 非本月为 null
  count: number;
}

/** 6×7 日期矩阵（周一首：offset = (getDay()+6)%7） */
const matrix = computed<Cell[][]>(() => {
  const y = viewYear.value;
  const m = viewMonth.value; // 1-12
  const first = new Date(y, m - 1, 1);
  const offset = (first.getDay() + 6) % 7;
  const cells: Cell[] = [];
  const start = new Date(y, m - 1, 1 - offset);
  for (let i = 0; i < 42; i++) {
    const d = new Date(start.getFullYear(), start.getMonth(), start.getDate() + i);
    const inMonth = d.getMonth() === m - 1;
    const key = inMonth ? fmtKey(d) : null;
    cells.push({ day: d.getDate(), key, count: key ? props.counts[key] || 0 : 0 });
  }
  const rows: Cell[][] = [];
  for (let r = 0; r < 6; r++) rows.push(cells.slice(r * 7, r * 7 + 7));
  return rows;
});

/** 色阶：count/max → 1-4 档透明度（主题色联动） */
function heatStyle(cell: Cell) {
  if (!cell.key || !cell.count || !monthMax.value) return {};
  const tier = Math.max(1, Math.ceil((cell.count / monthMax.value) * 4));
  const pct = [0, 22, 45, 68, 92][tier];
  return {
    background: `color-mix(in srgb, var(--color-primary) ${pct}%, transparent)`,
    color: tier >= 3 ? "var(--hero-cta-text, #fff)" : "var(--text-primary)",
  };
}

function onCellClick(cell: Cell) {
  if (!cell.key) return;
  emit("select", props.selected === cell.key ? null : cell.key);
}

/** 年视图：12 个迷你月历 */
const yearMonths = computed(() => {
  const arr: { month: number; matrix: Cell[][] }[] = [];
  for (let m = 1; m <= 12; m++) {
    const first = new Date(viewYear.value, m - 1, 1);
    const offset = (first.getDay() + 6) % 7;
    const cells: Cell[] = [];
    const start = new Date(viewYear.value, m - 1, 1 - offset);
    for (let i = 0; i < 42; i++) {
      const d = new Date(start.getFullYear(), start.getMonth(), start.getDate() + i);
      const inMonth = d.getMonth() === m - 1;
      const key = inMonth ? fmtKey(d) : null;
      cells.push({ day: d.getDate(), key, count: key ? props.counts[key] || 0 : 0 });
    }
    const rows: Cell[][] = [];
    for (let r = 0; r < 6; r++) rows.push(cells.slice(r * 7, r * 7 + 7));
    arr.push({ month: m, matrix: rows });
  }
  return arr;
});

const yearMax = computed(() => {
  let max = 0;
  for (const [date, c] of Object.entries(props.counts)) {
    if (date.startsWith(String(viewYear.value)) && c > max) max = c;
  }
  return max;
});

function yearHeat(key: string | null) {
  if (!key) return {};
  const c = props.counts[key] || 0;
  if (!c || !yearMax.value) return {};
  const tier = Math.max(1, Math.ceil((c / yearMax.value) * 4));
  const pct = [0, 25, 50, 75, 100][tier];
  return { background: `color-mix(in srgb, var(--color-primary) ${pct}%, transparent)` };
}

function pickFromYear(m: number, cell: Cell) {
  if (!cell.key) return;
  viewMonth.value = m;
  yearView.value = false;
  emit("select", props.selected === cell.key ? null : cell.key);
}

function shiftYear(delta: number) {
  viewYear.value += delta;
}

const WEEK = ["一", "二", "三", "四", "五", "六", "日"];
</script>

<template>
  <div class="memo-calendar">
    <div class="cal-head">
      <button class="cal-nav" title="上一月" @click="shiftMonth(-1)"><LeftOutlined /></button>
      <button class="cal-title" title="切换年视图" @click="yearView = !yearView">
        {{ viewYear }} 年 {{ String(viewMonth).padStart(2, "0") }} 月
      </button>
      <button class="cal-nav" title="下一月" @click="shiftMonth(1)"><RightOutlined /></button>
      <button class="cal-today" title="回到今天" @click="goToday">今</button>
    </div>

    <!-- 月视图 -->
    <template v-if="!yearView">
      <div class="cal-week">
        <span v-for="w in WEEK" :key="w">{{ w }}</span>
      </div>
      <div v-for="(row, ri) in matrix" :key="ri" class="cal-row">
        <button
          v-for="(cell, ci) in row"
          :key="ci"
          class="cal-cell"
          :class="{
            out: !cell.key,
            today: cell.key === todayKey,
            sel: cell.key && cell.key === selected,
            empty: cell.key && !cell.count,
          }"
          :style="heatStyle(cell)"
          :title="cell.key ? `${cell.key} · ${cell.count} 条` : ''"
          @click="onCellClick(cell)"
        >
          {{ cell.day }}
        </button>
      </div>
      <div class="cal-foot">
        {{ selected ? `已选 ${selected} · 再点取消` : "点日期筛选当日备忘录" }}
      </div>
    </template>

    <!-- 年视图：12 迷你月历 -->
    <template v-else>
      <div class="cal-head year-head">
        <button class="cal-nav" @click="shiftYear(-1)"><LeftOutlined /></button>
        <span class="cal-title static">{{ viewYear }} 年</span>
        <button class="cal-nav" @click="shiftYear(1)"><RightOutlined /></button>
      </div>
      <div class="year-grid">
        <div v-for="ym in yearMonths" :key="ym.month" class="year-month">
          <div class="ym-label">{{ ym.month }} 月</div>
          <div class="ym-days">
            <button
              v-for="(cell, ci) in ym.matrix.flat()"
              :key="ci"
              class="ym-cell"
              :class="{ out: !cell.key, today: cell.key === todayKey, sel: cell.key === selected }"
              :style="yearHeat(cell.key)"
              @click="pickFromYear(ym.month, cell)"
            >
              {{ cell.day }}
            </button>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.memo-calendar {
  padding: 10px 12px;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  background: var(--bg-input);
  user-select: none;
}

.cal-head {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 8px;
}
.cal-nav,
.cal-today {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  font-size: 11px;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.cal-nav:hover,
.cal-today:hover {
  color: var(--color-primary);
  border-color: var(--color-primary-border);
}
.cal-title {
  flex: 1;
  font-family: "Noto Serif SC", "Source Han Serif SC", "STZhongsong", serif;
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.03em;
  color: var(--text-primary);
  background: none;
  border: none;
  cursor: pointer;
  text-align: center;
  border-radius: var(--radius-xs);
  padding: 2px 6px;
}
.cal-title:hover {
  background: var(--color-hover);
}
.cal-title.static {
  cursor: default;
}
.cal-title.static:hover {
  background: none;
}
.year-head {
  margin-top: 4px;
}

.cal-week,
.cal-row {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 3px;
}
.cal-week {
  margin-bottom: 4px;
}
.cal-week span {
  text-align: center;
  font-size: 10.5px;
  color: var(--text-tertiary);
}
.cal-row {
  margin-bottom: 3px;
}

.cal-cell {
  position: relative;
  height: 30px;
  font-size: 11.5px;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid transparent;
  border-radius: var(--radius-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.cal-cell:hover {
  border-color: var(--color-primary-border);
}
.cal-cell.out {
  color: var(--text-tertiary);
  opacity: 0.4;
  cursor: default;
}
.cal-cell.today {
  border-color: var(--color-primary);
}
.cal-cell.sel {
  box-shadow: inset 0 0 0 1.5px var(--color-primary);
  font-weight: 700;
}

.cal-foot {
  margin-top: 8px;
  font-size: 11px;
  color: var(--text-tertiary);
  text-align: center;
}

/* 年视图 */
.year-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}
.year-month {
  min-width: 0;
}
.ym-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 3px;
}
.ym-days {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 1px;
}
.ym-cell {
  height: 17px;
  min-width: 0;
  padding: 0;
  font-size: 8.5px;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: 2px;
  cursor: pointer;
}
.ym-cell:hover {
  outline: 1px solid var(--color-primary-border);
}
.ym-cell.out {
  opacity: 0.3;
  cursor: default;
}
.ym-cell.today {
  outline: 1px solid var(--color-primary);
}
.ym-cell.sel {
  outline: 1.5px solid var(--color-primary);
}
</style>
