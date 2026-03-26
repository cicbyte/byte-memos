<script setup lang="ts">
/**
 * 仪表盘 /（书签数据概览中心）
 *
 * 三层结构：
 * 1. 概览统计卡（4 个核心指标：书签总数 / 收藏数 / 标签数 / 分类数）
 * 2. 标签词云（按使用频率缩放字号）
 * 3. 最近活动（最近添加 + 最常查看）
 *
 * 注：后端 stats 命令尚未提供书签维度统计，这里用 listBookmarks 在前端聚合
 * （过渡方案），待后端就绪后可改回 statsOverview。
 */
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import {
  LinkOutlined,
  StarOutlined,
  TagsOutlined,
  FolderOutlined,
} from "@ant-design/icons-vue";
import EmptyState from "@/components/common/EmptyState.vue";
import { api } from "@/api";
import type { Bookmark, TagStat } from "@/api/types";
import { useLayoutStore } from "@/stores/layout";

const router = useRouter();
const layout = useLayoutStore();

const bookmarkCount = ref(0);
const favoritedCount = ref(0);
const tagCount = ref(0);
const categoryCount = ref(0);
const tags = ref<TagStat[]>([]);
const recent = ref<Bookmark[]>([]);
const topViewed = ref<Bookmark[]>([]);
const loading = ref(true);

/** 词云展示的标签（最多 50 个，避免拥挤） */
const cloudTags = computed(() => tags.value.slice(0, 50));

/**
 * 词云样式：按 count 在最大/最小之间的比例缩放字号（12-24px），
 * 透明度也按热度区分（低频淡、高频浓）。
 */
function cloudStyle(count: number) {
  const max = Math.max(1, ...tags.value.map((t) => t.count));
  const ratio = count / max;
  return {
    fontSize: `${(12 + ratio * 12).toFixed(0)}px`,
    opacity: (0.55 + ratio * 0.45).toFixed(2),
    fontWeight: ratio > 0.5 ? 600 : 400,
  };
}

onMounted(async () => {
  try {
    // 拉取书签（一次拉较多用于聚合统计 / 最近 / 最常查看）
    const [bms, ts, cats] = await Promise.all([
      api.listBookmarks({ limit: 1000 }),
      api.tagStats(20),
      api.listCategories(),
    ]);
    bookmarkCount.value = bms.length;
    favoritedCount.value = bms.filter((b) => b.is_favorited).length;
    tags.value = ts;
    tagCount.value = ts.length;
    categoryCount.value = cats.length;
    // 最近添加（按 created_at 降序取前 6）
    recent.value = [...bms]
      .sort((a, b) => b.created_at.localeCompare(a.created_at))
      .slice(0, 6);
    // 最常查看（按 visit_count 降序取前 5）
    topViewed.value = [...bms]
      .sort((a, b) => b.visit_count - a.visit_count)
      .slice(0, 5);
    layout.setStatusItems([
      { key: "count", text: `共 ${bms.length} 个书签` },
    ]);
  } finally {
    loading.value = false;
  }
});
</script>

<template>
  <div class="dashboard">
    <h1 class="page-title">仪表盘</h1>

    <!-- 1. 概览统计卡（4 个核心指标） -->
    <div v-reveal-group class="overview-cards">
      <div class="ov-card" @click="router.push('/bookmarks')">
        <div class="ov-icon bookmark"><LinkOutlined /></div>
        <div class="ov-body">
          <div class="ov-value">{{ bookmarkCount }}</div>
          <div class="ov-label">书签</div>
        </div>
      </div>

      <div class="ov-card" @click="router.push('/favorites')">
        <div class="ov-icon favorite"><StarOutlined /></div>
        <div class="ov-body">
          <div class="ov-value">{{ favoritedCount }}</div>
          <div class="ov-label">收藏</div>
        </div>
      </div>

      <div class="ov-card" @click="router.push('/tags')">
        <div class="ov-icon tag"><TagsOutlined /></div>
        <div class="ov-body">
          <div class="ov-value">{{ tagCount }}</div>
          <div class="ov-label">标签</div>
        </div>
      </div>

      <div class="ov-card" @click="router.push('/categories')">
        <div class="ov-icon category"><FolderOutlined /></div>
        <div class="ov-body">
          <div class="ov-value">{{ categoryCount }}</div>
          <div class="ov-label">分类</div>
        </div>
      </div>
    </div>

    <!-- 2. 标签词云 -->
    <div v-reveal="0.28" class="chart-row">
      <div class="panel chart-panel">
        <div class="panel-header">
          <h2 class="panel-title">标签</h2>
          <a-button v-if="tags.length" type="link" size="small" @click="router.push('/tags')">全部 →</a-button>
        </div>
        <div v-if="tags.length" class="word-cloud">
          <span
            v-for="t in cloudTags"
            :key="t.tag"
            class="cloud-word"
            :style="cloudStyle(t.count)"
            :title="`${t.tag} · ${t.count} 项`"
            @click="router.push('/tags')"
          >#{{ t.tag }}</span>
        </div>
        <EmptyState v-else text="暂无标签" size="small" inline class="chart-empty" />
      </div>
    </div>

    <!-- 3. 最近活动（最近添加 + 最常查看） -->
    <div v-reveal="0.42" class="chart-row">
      <div class="panel">
        <div class="panel-header">
          <h2 class="panel-title">最近添加</h2>
          <a-button type="link" size="small" @click="router.push('/bookmarks')">全部 →</a-button>
        </div>
        <div v-if="recent.length" class="bm-list">
          <div
            v-for="b in recent"
            :key="b.short_id"
            class="bm-row"
            :title="b.url"
            @click="router.push(`/bookmarks?id=${b.short_id}`)"
          >
            <LinkOutlined class="bm-icon" />
            <span class="bm-title">{{ b.title || b.url }}</span>
            <span v-if="b.category" class="bm-meta">{{ b.category }}</span>
          </div>
        </div>
        <EmptyState v-else-if="!loading" text="还没有书签" size="small" inline class="chart-empty" />
      </div>

      <div class="panel">
        <div class="panel-header">
          <h2 class="panel-title">最常查看</h2>
        </div>
        <div class="view-list">
          <div
            v-for="(b, i) in topViewed"
            :key="b.short_id"
            class="view-row"
            :title="b.url"
            @click="router.push(`/bookmarks?id=${b.short_id}`)"
          >
            <span class="rank">{{ i + 1 }}</span>
            <span class="view-name">{{ b.title || b.url }}</span>
            <span class="view-count">{{ b.visit_count }} 次</span>
          </div>
          <EmptyState v-if="!topViewed.length" text="暂无数据" size="small" inline class="chart-empty" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
  height: 100%;
  padding: var(--spacing-md);
  overflow-y: auto;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: var(--text-primary);
}

/* ============ 概览统计卡 ============ */
.overview-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: var(--spacing-md);
}
.ov-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-lg);
  cursor: pointer;
  transition: border-color var(--transition-fast), transform var(--transition-fast);
}
.ov-card:hover {
  border-color: var(--color-primary-border);
  transform: translateY(-2px);
}
.ov-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: var(--radius-md);
  font-size: 20px;
  flex-shrink: 0;
}
.ov-icon.bookmark { color: var(--color-primary); background: var(--color-primary-bg); }
.ov-icon.favorite { color: var(--color-warning); background: var(--warning-bg); }
.ov-icon.tag { color: var(--color-success); background: var(--success-bg); }
.ov-icon.category { color: var(--color-info); background: var(--info-bg); }
.ov-body {
  flex: 1;
  min-width: 0;
}
.ov-value {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  font-family: "JetBrains Mono", monospace;
  line-height: 1.2;
}
.ov-label {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-top: 2px;
}

/* ============ 图表 / 列表面板通用 ============ */
.chart-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: var(--spacing-md);
}
.panel {
  padding: 16px;
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-lg);
}
.panel-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 12px;
}
.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.panel-header .panel-title {
  margin-bottom: 0;
}
.chart-empty {
  height: 200px;
}

/* 标签词云 */
.word-cloud {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 10px;
  min-height: 200px;
  padding: 4px 0;
}
.cloud-word {
  color: var(--color-primary);
  cursor: pointer;
  line-height: 1.6;
  transition: opacity var(--transition-fast), color var(--transition-fast);
  user-select: none;
}
.cloud-word:hover {
  opacity: 1 !important;
  color: var(--color-primary-active);
}

/* 最近添加（书签行） */
.bm-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.bm-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 12px;
  transition: background var(--transition-fast);
}
.bm-row:hover {
  background: var(--bg-elevated);
}
.bm-icon {
  color: var(--color-primary);
  flex-shrink: 0;
}
.bm-title {
  flex: 1;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bm-meta {
  color: var(--text-tertiary);
  font-size: 11px;
  flex-shrink: 0;
}

/* 最常查看 */
.view-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.view-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 12px;
  transition: background var(--transition-fast);
}
.view-row:hover {
  background: var(--bg-elevated);
}
.rank {
  color: var(--text-tertiary);
  width: 16px;
  font-family: "JetBrains Mono", monospace;
}
.view-name {
  flex: 1;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.view-count {
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
}
</style>
