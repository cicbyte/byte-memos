//! 笔记搜索（search_all 统一搜索）+ 统计 + 回收站计数。

use rusqlite::params;

use crate::model::*;
use crate::CoreResult;

use super::Database;
use super::common::build_fts_query;
use super::memos::search_memos_fallback;

impl Database {
    /// 回收站条目总数（软删除的笔记数，stats 用）
    pub fn trashed_count(&self) -> CoreResult<i64> {
        let conn = self.lock()?;
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memo WHERE deleted_at IS NOT NULL",
            [],
            |r| r.get(0),
        ).unwrap_or(0);
        Ok(n)
    }

    // ============ 搜索 ============

    /// 笔记全文搜索（FTS5 bm25 排名）。
    /// 保留以兼容旧调用方；统一搜索入口请用 [`Database::search_all`]。
    pub fn search(&self, query: &str, limit: i64, offset: i64) -> CoreResult<Vec<SearchHit>> {
        let conn = self.lock()?;
        let fts_query = build_fts_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = conn.prepare(
            "SELECT m.short_id, m.title,
                    snippet(memo_fts, 1, '<mark>', '</mark>', '…', 16) AS snip,
                    bm25(memo_fts) AS rank,
                    (SELECT COALESCE(group_concat(t.name, ', '),'')
                       FROM memo_tag mt JOIN tags t ON t.id = mt.tag_id
                       WHERE mt.memo_id = m.short_id) AS tags_str
             FROM memo_fts f
             JOIN memo m ON m.id = f.rowid
             WHERE memo_fts MATCH ? AND m.deleted_at IS NULL
             ORDER BY rank
             LIMIT ? OFFSET ?",
        )?;
        let rows = stmt
            .query_map(params![fts_query, limit, offset], |r| {
                let short_id: String = r.get(0)?;
                let title: String = r.get::<_, Option<String>>(1)?.unwrap_or_default();
                let snip: String = r.get::<_, Option<String>>(2)?.unwrap_or_default();
                let rank: f64 = r.get::<_, Option<f64>>(3)?.unwrap_or(0.0);
                let tags_str: String = r.get::<_, Option<String>>(4)?.unwrap_or_default();
                let matched_tags: Vec<String> = if tags_str.is_empty() {
                    Vec::new()
                } else {
                    tags_str.split(", ").map(|s| s.to_string()).collect()
                };
                Ok(SearchHit {
                    short_id,
                    title,
                    snippet: snip,
                    rank,
                    matched_tags,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    // ============ 统计 ============

    /// 总体统计（dashboard）：笔记维度。
    ///
    /// total_size 由 service 层从文件系统统计后覆盖（DB 无法直接得知磁盘占用）。
    pub fn stats_overview(&self) -> CoreResult<StatsOverview> {
        let conn = self.lock()?;
        let memo_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memo WHERE deleted_at IS NULL", [], |r| r.get(0))
            .unwrap_or(0);
        let tag_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
            .unwrap_or(0);
        let pinned_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memo WHERE pinned = 1 AND deleted_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        Ok(StatsOverview {
            memo_count,
            tag_count,
            category_count: 0, // 由 service 层用 category_count() 覆盖
            total_size: 0,     // 由 service 层从文件系统统计
            pinned_count,
            trashed_count: 0,  // 由 service 层用 trashed_count() 覆盖
        })
    }

    /// 标签使用统计（按引用数降序，仅聚合笔记关联，排除软删除笔记）
    pub fn tag_stats(&self, limit: i64) -> CoreResult<Vec<TagStat>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT t.name,
                (SELECT COUNT(*) FROM memo_tag mt
                   JOIN memo m ON m.short_id = mt.memo_id
                 WHERE mt.tag_id = t.id AND m.deleted_at IS NULL) AS cnt
             FROM tags t
             WHERE cnt > 0
             ORDER BY cnt DESC, t.name LIMIT ?",
        )?;
        let rows = stmt
            .query_map(params![limit], |r| {
                Ok(TagStat {
                    tag: r.get(0)?,
                    count: r.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// WAL checkpoint：把 -wal 日志刷入主库，备份前调用确保一致性
    pub fn wal_checkpoint(&self) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        Ok(())
    }

    // ============ 统一搜索（笔记：FTS + 中文 LIKE 兜底）============

    /// 统一搜索（笔记类型）。
    ///
    /// 先走 FTS5（英文/前缀，bm25 排序），再用 LIKE 兜底（中文/模糊，整句 `%query%` 匹配
    /// title/content/tags）。LIKE 命中去重后追加在 FTS 命中之后，按 rank 排序。
    ///
    /// - FTS 命中 rank 为 bm25 负数（越小越相关）；LIKE 兜底 rank 统一 0.0（排在 FTS 之后）。
    pub fn search_all(&self, query: &str, limit: i64, offset: i64) -> CoreResult<Vec<UnifiedSearchHit>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.lock()?;
        let like_pat = format!("%{trimmed}%");

        // 多取一些以支持客户端分页（offset + limit）
        let need = (limit + offset.max(0)) as usize;
        let per_type = need.max(2);

        let mut hits = search_memos_fallback(&conn, trimmed, &like_pat, per_type)?;

        // 合并后按 rank 升序（FTS bm25 负数优先，LIKE 0.0 在后）
        hits.sort_by(|a, b| a.rank.partial_cmp(&b.rank).unwrap_or(std::cmp::Ordering::Equal));
        // 客户端分页：跳过 offset 条，取 limit 条
        let start = (offset.max(0) as usize).min(hits.len());
        let end = (start + limit as usize).min(hits.len());
        Ok(hits[start..end].to_vec())
    }

    // ============ 正则表达式搜索 ============

    /// 正则表达式搜索（在 Rust 中做正则匹配）
    ///
    /// 在 title 和 content 字段上执行正则匹配，返回匹配的笔记。
    pub fn search_regex(&self, pattern: &str, limit: i64, offset: i64) -> CoreResult<Vec<SearchHit>> {
        // 验证正则表达式是否有效
        let re = regex::Regex::new(pattern)
            .map_err(|e| crate::CoreError::Other(anyhow::anyhow!("无效的正则表达式：{e}")))?;

        let conn = self.lock()?;
        // 先获取所有活跃笔记
        let mut stmt = conn.prepare(
            "SELECT m.short_id, m.title, m.content,
                    (SELECT COALESCE(group_concat(t.name, ', '),'')
                       FROM memo_tag mt JOIN tags t ON t.id = mt.tag_id
                       WHERE mt.memo_id = m.short_id) AS tags_str
             FROM memo m
             WHERE m.deleted_at IS NULL
             ORDER BY m.updated_at DESC",
        )?;

        let mut hits = Vec::new();
        let rows = stmt.query_map([], |r| {
            let short_id: String = r.get(0)?;
            let title: String = r.get::<_, Option<String>>(1)?.unwrap_or_default();
            let content: String = r.get::<_, Option<String>>(2)?.unwrap_or_default();
            let tags_str: String = r.get::<_, Option<String>>(3)?.unwrap_or_default();
            let matched_tags: Vec<String> = if tags_str.is_empty() {
                Vec::new()
            } else {
                tags_str.split(", ").map(|s| s.to_string()).collect()
            };
            Ok((short_id, title, content, matched_tags))
        })?;

        for row in rows {
            let (short_id, title, content, matched_tags) = row?;
            // 在 Rust 中做正则匹配
            if re.is_match(&title) || re.is_match(&content) {
                let snippet = if content.len() > 200 {
                    format!("{}...", &content[..200])
                } else {
                    content
                };
                hits.push(SearchHit {
                    short_id,
                    title,
                    snippet,
                    rank: 0.0,
                    matched_tags,
                });
            }
        }

        // 分页
        let start = (offset.max(0) as usize).min(hits.len());
        let end = (start + limit as usize).min(hits.len());
        Ok(hits[start..end].to_vec())
    }
}
