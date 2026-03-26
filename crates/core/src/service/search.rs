//! 搜索（笔记 FTS 搜索 + 统一搜索）与统计概览、标签统计。
use super::*;
use std::path::Path;

impl super::ProjectService {
    // ============ 搜索 ============

    /// 笔记全文搜索（返回 SearchHit）
    pub fn search(&self, query: &str, limit: i64, offset: i64) -> CoreResult<Vec<SearchHit>> {
        self.db.search(query, limit, offset)
    }

    /// 统一搜索（笔记类型）。
    ///
    /// 英文/前缀走 FTS5（bm25 排序），中文/模糊走 LIKE 兜底（title/content/tags）。
    /// 详见 [`crate::db::Database::search_all`]。
    pub fn search_all(&self, query: &str, limit: i64, offset: i64) -> CoreResult<Vec<UnifiedSearchHit>> {
        self.db.search_all(query, limit, offset)
    }

    /// 正则表达式搜索
    ///
    /// 使用正则表达式在标题和内容中搜索。
    pub fn search_regex(&self, pattern: &str, limit: i64, offset: i64) -> CoreResult<Vec<SearchHit>> {
        self.db.search_regex(pattern, limit, offset)
    }

    // ============ 统计 ============

    /// 统计概览（笔记维度 + 共享的标签/分类/存储/置顶/回收站计数）
    pub fn stats_overview(&self) -> CoreResult<StatsOverview> {
        let mut s = self.db.stats_overview()?;
        s.category_count = self.db.category_count()?;
        // 总占用：遍历 data/ 目录统计
        s.total_size = compute_data_size(self.storage.root())?;
        s.trashed_count = self.db.trashed_count()?;
        Ok(s)
    }

    /// 标签使用统计（仅笔记关联）
    pub fn tag_stats(&self, limit: i64) -> CoreResult<Vec<TagStat>> {
        self.db.tag_stats(limit)
    }

    /// 按本地日期聚合的备忘录日计数（日历热力图数据源）
    pub fn list_memo_daily_counts(&self) -> CoreResult<Vec<crate::db::stats::DailyCount>> {
        self.db.list_memo_daily_counts()
    }
}

/// 递归统计 data/ 目录下所有文件总大小
fn compute_data_size(data_root: &Path) -> CoreResult<u64> {
    if !data_root.exists() {
        return Ok(0);
    }
    let mut total: u64 = 0;
    for entry in walkdir::WalkDir::new(data_root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    Ok(total)
}
