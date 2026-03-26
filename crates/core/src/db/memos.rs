//! 备忘录（memos）的 CRUD、过滤列表、FTS 同步、回收站。
//!
//! 与 bookmarks 的差异：content 直接存 DB（不落盘），故 FTS 用 `INSERT ... SELECT`
//! 直接同步 DB 列，service 层无需读写内容文件。short_id 去重由部分唯一索引保证。

use rusqlite::{params, Connection, OptionalExtension};

use crate::model::*;
use crate::{CoreError, CoreResult};

use super::Database;
use super::common::{build_fts_query, join_where, load_category_names_batch, load_rel_names_batch, now_iso};

impl Database {
    // ============ 备忘录 ============

    /// 该备忘录 shortId 是否已存在
    pub fn memo_id_exists(&self, short_id: &str) -> CoreResult<bool> {
        let conn = self.lock()?;
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memo WHERE short_id = ?",
            params![short_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// 插入备忘录记录
    pub fn insert_memo(&self, m: &Memo) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO memo
                (short_id, title, content, visibility, pinned, category_id,
                 created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                m.short_id,
                m.title,
                m.content,
                m.visibility,
                m.pinned as i64,
                m.category_id,
                m.created_at,
                m.updated_at,
            ],
        )?;
        Ok(())
    }

    /// 替换某备忘录的全部标签关联
    pub fn set_memo_tags(&self, short_id: &str, tag_ids: &[i64]) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "DELETE FROM memo_tag WHERE memo_id = ?",
            params![short_id],
        )?;
        let mut stmt =
            conn.prepare("INSERT INTO memo_tag(memo_id, tag_id) VALUES(?,?)")?;
        for tid in tag_ids {
            stmt.execute(params![short_id, tid])?;
        }
        Ok(())
    }

    /// 同步某备忘录的 FTS 行（直接从 DB 列读取，无需外部内容）
    pub fn sync_memo_fts(&self, short_id: &str) -> CoreResult<()> {
        let conn = self.lock()?;
        // FTS5 内容表可直接用 DELETE FROM ... WHERE rowid = ?
        let rowid: i64 = conn.query_row(
            "SELECT id FROM memo WHERE short_id = ?",
            params![short_id],
            |r| r.get(0),
        )?;
        conn.execute("DELETE FROM memo_fts WHERE rowid = ?", params![rowid])?;
        conn.execute(
            "INSERT INTO memo_fts(rowid, title, content, tags)
             SELECT m.id, m.title, m.content,
                (SELECT COALESCE(group_concat(t.name, ' '),'') FROM memo_tag r
                   JOIN tags t ON t.id = r.tag_id WHERE r.memo_id = m.short_id)
             FROM memo m WHERE m.short_id = ?",
            params![short_id],
        )?;
        Ok(())
    }

    /// 按 shortId 取单个备忘录（仅活跃）
    pub fn get_memo(&self, short_id: &str) -> CoreResult<Option<Memo>> {
        let conn = self.lock()?;
        let loaded = conn
            .query_row(
                "SELECT short_id, title, content, visibility, pinned, category_id,
                        created_at, updated_at
                 FROM memo WHERE short_id = ? AND deleted_at IS NULL",
                params![short_id],
                |r| load_memo_row(r),
            )
            .optional()?;
        let Some(mut m) = loaded else { return Ok(None) };
        attach_memo_relations(&conn, &mut m)?;
        Ok(Some(m))
    }

    /// 列出备忘录（带过滤 + 批量加载关联，无 N+1）
    pub fn list_memos(&self, filter: &MemoFilter) -> CoreResult<Vec<Memo>> {
        let conn = self.lock()?;
        let (clauses, params) = memo_where(filter);
        let where_sql = join_where(&clauses);
        let mut sql = format!(
            "SELECT short_id, title, content, visibility, pinned, category_id,
                    created_at, updated_at
             FROM memo {where_sql} ORDER BY created_at DESC"
        );
        if let Some(l) = filter.limit {
            sql.push_str(&format!(" LIMIT {l}"));
            if let Some(o) = filter.offset {
                sql.push_str(&format!(" OFFSET {o}"));
            }
        }
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows: Vec<Memo> = stmt
            .query_map(param_refs.as_slice(), |r| load_memo_row(r))?
            .filter_map(|r| r.ok())
            .collect();
        if rows.is_empty() {
            return Ok(rows);
        }
        let ids: Vec<String> = rows.iter().map(|m| m.short_id.clone()).collect();
        let mut tags_map = load_rel_names_batch(
            &conn,
            "memo_tag",
            "memo_id",
            "tags",
            "tag_id",
            &ids,
        )?;
        let cids: Vec<i64> = rows.iter().filter_map(|m| m.category_id).collect();
        let cat_map = load_category_names_batch(&conn, &cids)?;
        let mut out = Vec::with_capacity(rows.len());
        for mut m in rows {
            m.tags = tags_map.remove(&m.short_id).unwrap_or_default();
            if let Some(cid) = m.category_id {
                m.category = cat_map.get(&cid).cloned();
            }
            out.push(m);
        }
        Ok(out)
    }

    /// 备忘录总数（按 filter 计数，供分页 total 用）
    pub fn count_memos(&self, filter: &MemoFilter) -> CoreResult<i64> {
        let conn = self.lock()?;
        let (clauses, params) = memo_where(filter);
        let where_sql = join_where(&clauses);
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let n: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM memo {where_sql}"),
            param_refs.as_slice(),
            |r| r.get(0),
        )?;
        Ok(n)
    }

    pub fn set_memo_pinned(&self, short_id: &str, pinned: bool) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE memo SET pinned = ?, updated_at = ? WHERE short_id = ?",
            params![pinned as i64, now_iso(), short_id],
        )?;
        Ok(())
    }

    /// 更新备忘录元数据（title/content/visibility/category_id/pinned）
    pub fn update_memo_meta(
        &self,
        short_id: &str,
        title: &str,
        content: &str,
        visibility: &str,
        category_id: Option<i64>,
        pinned: bool,
    ) -> CoreResult<()> {
        let conn = self.lock()?;
        let now = now_iso();
        let affected = conn.execute(
            "UPDATE memo SET title = ?, content = ?, visibility = ?, category_id = ?,
                pinned = ?, updated_at = ?
             WHERE short_id = ?",
            params![title, content, visibility, category_id, pinned as i64, now, short_id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound(short_id.into()));
        }
        Ok(())
    }

    /// 物理删除备忘录（关联表 CASCADE，FTS 手动删）——仅 purge 调用
    pub fn delete_memo(&self, short_id: &str) -> CoreResult<()> {
        let conn = self.lock()?;
        // FTS5 内容表需先查出整数 rowid
        let rowid: i64 = conn.query_row(
            "SELECT id FROM memo WHERE short_id = ?",
            params![short_id],
            |r| r.get(0),
        )?;
        let affected = conn.execute("DELETE FROM memo WHERE short_id = ?", params![short_id])?;
        if affected == 0 {
            return Err(CoreError::NotFound(short_id.into()));
        }
        conn.execute("DELETE FROM memo_fts WHERE rowid = ?", params![rowid])?;
        Ok(())
    }

    /// 移入回收站（软删除：打 deleted_at 标记）
    pub fn trash_memo(&self, short_id: &str) -> CoreResult<()> {
        let conn = self.lock()?;
        // FTS5 内容表需先查出整数 rowid
        let rowid: i64 = conn.query_row(
            "SELECT id FROM memo WHERE short_id = ?",
            params![short_id],
            |r| r.get(0),
        )?;
        let now = now_iso();
        let affected = conn.execute(
            "UPDATE memo SET deleted_at = ? WHERE short_id = ? AND deleted_at IS NULL",
            params![now, short_id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound(short_id.into()));
        }
        // FTS5 内容表直接 DELETE FROM（数据行保留，恢复时由 service 重建 FTS）
        conn.execute("DELETE FROM memo_fts WHERE rowid = ?", params![rowid])?;
        Ok(())
    }

    /// 从回收站恢复（清 deleted_at 标记；FTS 由 service 重建）
    pub fn restore_memo(&self, short_id: &str) -> CoreResult<()> {
        let conn = self.lock()?;
        let affected = conn.execute(
            "UPDATE memo SET deleted_at = NULL WHERE short_id = ?",
            params![short_id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound(short_id.into()));
        }
        Ok(())
    }

    /// 备忘录总数（活跃）
    pub fn memo_count(&self) -> CoreResult<i64> {
        let conn = self.lock()?;
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memo WHERE deleted_at IS NULL",
            [],
            |r| r.get(0),
        )?;
        Ok(n)
    }

    /// 备忘录全文搜索（bm25 排名 + content 列高亮 snippet）。
    /// 仅走 FTS5（英文/前缀可靠）；中文/模糊兜底由 search_all 的 fallback 统一处理（F3 集成）。
    pub fn search_memos(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> CoreResult<Vec<MemoSearchHit>> {
        let conn = self.lock()?;
        let fts_query = build_fts_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = conn.prepare(
            "SELECT s.short_id, s.title,
                    snippet(memo_fts, 1, '<mark>', '</mark>', '…', 16) AS snip,
                    bm25(memo_fts) AS rank,
                    (SELECT c.name FROM project_categories c WHERE c.id = s.category_id)
             FROM memo_fts f
             JOIN memo s ON s.id = f.rowid
             WHERE memo_fts MATCH ? AND s.deleted_at IS NULL
             ORDER BY rank LIMIT ? OFFSET ?",
        )?;
        let hits: Vec<(String, String, Option<String>, Option<f64>, Option<String>)> = stmt
            .query_map(params![fts_query, limit, offset], |r| {
                Ok((
                    r.get(0)?,
                    r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();
        // 批量加载标签（避免 N+1）
        let ids: Vec<String> = hits.iter().map(|h| h.0.clone()).collect();
        let mut tags_map = load_rel_names_batch(&conn, "memo_tag", "memo_id", "tags", "tag_id", &ids)?;
        let rows = hits
            .into_iter()
            .map(|(sid, title, snip, rank, cat)| MemoSearchHit {
                short_id: sid.clone(),
                title,
                snippet: snip,
                category: cat,
                tags: tags_map.remove(&sid).unwrap_or_default(),
                rank,
            })
            .collect();
        Ok(rows)
    }

    /// 列出回收站内的备忘录
    pub fn list_trashed_memos(&self) -> CoreResult<Vec<TrashedItem>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT m.short_id, m.title, m.content, m.deleted_at, m.created_at,
                    (SELECT c.name FROM project_categories c WHERE c.id = m.category_id)
             FROM memo m WHERE m.deleted_at IS NOT NULL
             ORDER BY m.deleted_at DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(TrashedItem {
                    short_id: r.get(0)?,
                    item_type: ItemType::Memo,
                    title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    description: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    deleted_at: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    created_at: r.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    category: r.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }
}

/// 构建 memo 的 WHERE 子句片段与参数（list / count 共用）。
fn memo_where(filter: &MemoFilter) -> (Vec<String>, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut clauses = vec!["deleted_at IS NULL".to_string()];
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if filter.favorited_only {
        clauses.push("pinned = 1".into());
    }
    if let Some(t) = &filter.tag {
        clauses.push(
            "short_id IN (SELECT memo_id FROM memo_tag r \
             JOIN tags t ON t.id = r.tag_id WHERE t.name = ?)"
                .into(),
        );
        params.push(Box::new(t.clone()));
    }
    if let Some(c) = &filter.category {
        clauses.push("category_id IN (SELECT id FROM project_categories WHERE name = ?)".into());
        params.push(Box::new(c.clone()));
    }
    (clauses, params)
}

/// 从一行装载 Memo（不含关联）
fn load_memo_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Memo> {
    Ok(Memo {
        short_id: r.get(0)?,
        title: r.get(1)?,
        content: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
        visibility: r.get::<_, Option<String>>(3)?.unwrap_or_else(|| "PRIVATE".into()),
        pinned: r.get::<_, i64>(4)? != 0,
        category_id: r.get(5)?,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
        tags: Vec::new(),
        category: None,
        deleted_at: None,
    })
}

/// 给 Memo 附上标签 + 分类名
fn attach_memo_relations(conn: &Connection, m: &mut Memo) -> CoreResult<()> {
    let mut stmt = conn.prepare(
        "SELECT t.name FROM memo_tag r JOIN tags t ON t.id = r.tag_id
         WHERE r.memo_id = ? ORDER BY t.name",
    )?;
    m.tags = stmt
        .query_map(params![m.short_id], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    if let Some(cid) = m.category_id {
        m.category = conn
            .query_row(
                "SELECT name FROM project_categories WHERE id = ?",
                params![cid],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
    }
    Ok(())
}

/// 备忘录搜索：FTS 命中（含高亮 snippet）+ LIKE 兜底（中文/模糊：title/content/tags）。
/// 供 search_all 跨类型统一搜索调用，返回 UnifiedSearchHit（item_type=Memo）。
pub(super) fn search_memos_fallback(
    conn: &Connection,
    query: &str,
    like_pat: &str,
    limit: usize,
) -> CoreResult<Vec<UnifiedSearchHit>> {
    let mut out = Vec::new();
    let fts_query = build_fts_query(query);
    if !fts_query.is_empty() {
        let mut stmt = conn.prepare(
            "SELECT s.short_id, s.title,
                    snippet(memo_fts, 1, '<mark>', '</mark>', '…', 16) AS snip,
                    bm25(memo_fts) AS rank,
                    (SELECT c.name FROM project_categories c WHERE c.id = s.category_id)
             FROM memo_fts f JOIN memo s ON s.id = f.rowid
             WHERE memo_fts MATCH ? AND s.deleted_at IS NULL
             ORDER BY rank LIMIT ?",
        )?;
        let rows = stmt
            .query_map(params![fts_query, limit as i64], |r| {
                Ok(UnifiedSearchHit {
                    short_id: r.get(0)?,
                    item_type: ItemType::Memo,
                    title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    snippet: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    category: r.get(4)?,
                    rank: r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                })
            })?
            .filter_map(|r| r.ok());
        for r in rows {
            out.push(r);
        }
    }
    if out.len() < limit {
        let already: Vec<String> = out.iter().map(|h| h.short_id.clone()).collect();
        let placeholder = if already.is_empty() {
            "''".to_string()
        } else {
            format!("'{}'", already.join("','"))
        };
        let sql = format!(
            "SELECT s.short_id, s.title,
                    (SELECT c.name FROM project_categories c WHERE c.id = s.category_id)
             FROM memo s
             WHERE s.deleted_at IS NULL
               AND s.short_id NOT IN ({placeholder})
               AND (s.title LIKE ? OR s.content LIKE ?
                    OR s.short_id IN (SELECT r.memo_id FROM memo_tag r
                                      JOIN tags t ON t.id = r.tag_id WHERE t.name LIKE ?))
             ORDER BY s.created_at DESC LIMIT ?"
        );
        let mut stmt = conn.prepare(&sql)?;
        let remaining = limit - out.len();
        let rows = stmt
            .query_map(params![like_pat, like_pat, like_pat, remaining as i64], |r| {
                Ok(UnifiedSearchHit {
                    short_id: r.get(0)?,
                    item_type: ItemType::Memo,
                    title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    snippet: String::new(),
                    category: r.get(2)?,
                    rank: 0.0,
                })
            })?
            .filter_map(|r| r.ok());
        for r in rows {
            out.push(r);
        }
    }
    Ok(out)
}
