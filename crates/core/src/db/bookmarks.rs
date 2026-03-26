//! 书签（bookmarks）的 CRUD、过滤列表、FTS 同步、访问计数、回收站。
//!
//! 与 snippets 的差异：description 直接存 DB（不落盘），故 FTS 用 `INSERT ... SELECT`
//! 直接同步 DB 列，service 层无需读写内容文件。url 去重由部分唯一索引保证。

use rusqlite::{params, Connection, OptionalExtension};

use crate::model::*;
use crate::{CoreError, CoreResult};

use super::Database;
use super::common::{build_fts_query, join_where, load_category_names_batch, load_rel_names_batch, now_iso};

impl Database {
    // ============ 书签 ============

    /// 该书签 shortId 是否已存在
    pub fn bookmark_id_exists(&self, short_id: &str) -> CoreResult<bool> {
        let conn = self.lock()?;
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM bookmarks WHERE short_id = ?",
            params![short_id],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// 按 url 取活跃书签（去重 / get_by_url 用）
    pub fn get_bookmark_by_url(&self, url: &str) -> CoreResult<Option<Bookmark>> {
        let conn = self.lock()?;
        let loaded = conn
            .query_row(
                "SELECT short_id, title, url, description, category_id, favicon_path,
                        visit_count, is_archived, is_favorited, created_at, updated_at
                 FROM bookmarks WHERE url = ? AND deleted_at IS NULL",
                params![url],
                |r| load_bookmark_row(r),
            )
            .optional()?;
        let Some(mut b) = loaded else { return Ok(None) };
        attach_bookmark_relations(&conn, &mut b)?;
        Ok(Some(b))
    }

    /// 插入书签记录
    pub fn insert_bookmark(&self, b: &Bookmark) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO bookmarks
                (short_id, title, url, description, category_id, favicon_path,
                 visit_count, is_archived, is_favorited, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                b.short_id,
                b.title,
                b.url,
                b.description,
                b.category_id,
                b.favicon_path,
                b.visit_count,
                b.is_archived as i64,
                b.is_favorited as i64,
                b.created_at,
                b.updated_at,
            ],
        )?;
        Ok(())
    }

    /// 替换某书签的全部标签关联
    pub fn set_bookmark_tags(&self, short_id: &str, tag_ids: &[i64]) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "DELETE FROM bookmark_tag_rel WHERE bookmark_id = ?",
            params![short_id],
        )?;
        let mut stmt =
            conn.prepare("INSERT INTO bookmark_tag_rel(bookmark_id, tag_id) VALUES(?,?)")?;
        for tid in tag_ids {
            stmt.execute(params![short_id, tid])?;
        }
        Ok(())
    }

    /// 同步某书签的 FTS 行（直接从 DB 列读取，无需外部内容）
    pub fn sync_bookmark_fts(&self, short_id: &str) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute("DELETE FROM bookmarks_fts WHERE short_id = ?", params![short_id])?;
        conn.execute(
            "INSERT INTO bookmarks_fts(short_id, title, url, description, tags)
             SELECT b.short_id, b.title, b.url, b.description,
                (SELECT COALESCE(group_concat(t.name, ' '),'') FROM bookmark_tag_rel r
                   JOIN tags t ON t.id = r.tag_id WHERE r.bookmark_id = b.short_id)
             FROM bookmarks b WHERE b.short_id = ?",
            params![short_id],
        )?;
        Ok(())
    }

    /// 按 shortId 取单个书签（仅活跃）
    pub fn get_bookmark(&self, short_id: &str) -> CoreResult<Option<Bookmark>> {
        let conn = self.lock()?;
        let loaded = conn
            .query_row(
                "SELECT short_id, title, url, description, category_id, favicon_path,
                        visit_count, is_archived, is_favorited, created_at, updated_at
                 FROM bookmarks WHERE short_id = ? AND deleted_at IS NULL",
                params![short_id],
                |r| load_bookmark_row(r),
            )
            .optional()?;
        let Some(mut b) = loaded else { return Ok(None) };
        attach_bookmark_relations(&conn, &mut b)?;
        Ok(Some(b))
    }

    /// 列出书签（带过滤 + 批量加载关联，无 N+1）
    pub fn list_bookmarks(&self, filter: &BookmarkFilter) -> CoreResult<Vec<Bookmark>> {
        let conn = self.lock()?;
        let (clauses, params) = bookmark_where(filter);
        let where_sql = join_where(&clauses);
        let mut sql = format!(
            "SELECT short_id, title, url, description, category_id, favicon_path,
                    visit_count, is_archived, is_favorited, created_at, updated_at
             FROM bookmarks {where_sql} ORDER BY created_at DESC"
        );
        if let Some(l) = filter.limit {
            sql.push_str(&format!(" LIMIT {l}"));
            if let Some(o) = filter.offset {
                sql.push_str(&format!(" OFFSET {o}"));
            }
        }
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows: Vec<Bookmark> = stmt
            .query_map(param_refs.as_slice(), |r| load_bookmark_row(r))?
            .filter_map(|r| r.ok())
            .collect();
        if rows.is_empty() {
            return Ok(rows);
        }
        let ids: Vec<String> = rows.iter().map(|b| b.short_id.clone()).collect();
        let mut tags_map = load_rel_names_batch(
            &conn,
            "bookmark_tag_rel",
            "bookmark_id",
            "tags",
            "tag_id",
            &ids,
        )?;
        let cids: Vec<i64> = rows.iter().filter_map(|b| b.category_id).collect();
        let cat_map = load_category_names_batch(&conn, &cids)?;
        let mut out = Vec::with_capacity(rows.len());
        for mut b in rows {
            b.tags = tags_map.remove(&b.short_id).unwrap_or_default();
            if let Some(cid) = b.category_id {
                b.category = cat_map.get(&cid).cloned();
            }
            out.push(b);
        }
        Ok(out)
    }

    /// 书签总数（按 filter 计数，供分页 total 用）
    pub fn count_bookmarks(&self, filter: &BookmarkFilter) -> CoreResult<i64> {
        let conn = self.lock()?;
        let (clauses, params) = bookmark_where(filter);
        let where_sql = join_where(&clauses);
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let n: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM bookmarks {where_sql}"),
            param_refs.as_slice(),
            |r| r.get(0),
        )?;
        Ok(n)
    }

    /// 自增访问次数（「打开浏览器」时调用）
    pub fn increment_bookmark_visit(&self, short_id: &str) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE bookmarks SET visit_count = visit_count + 1, updated_at = ? WHERE short_id = ?",
            params![now_iso(), short_id],
        )?;
        Ok(())
    }

    pub fn set_bookmark_favorited(&self, short_id: &str, favorited: bool) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE bookmarks SET is_favorited = ?, updated_at = ? WHERE short_id = ?",
            params![favorited as i64, now_iso(), short_id],
        )?;
        Ok(())
    }

    pub fn set_bookmark_archived(&self, short_id: &str, archived: bool) -> CoreResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE bookmarks SET is_archived = ?, updated_at = ? WHERE short_id = ?",
            params![archived as i64, now_iso(), short_id],
        )?;
        Ok(())
    }

    /// 更新书签元数据（title/url/description/category_id/favicon_path/is_archived）
    pub fn update_bookmark_meta(
        &self,
        short_id: &str,
        title: &str,
        url: &str,
        description: &str,
        category_id: Option<i64>,
        favicon_path: &str,
        is_archived: bool,
    ) -> CoreResult<()> {
        let conn = self.lock()?;
        let now = now_iso();
        let affected = conn.execute(
            "UPDATE bookmarks SET title = ?, url = ?, description = ?, category_id = ?,
                favicon_path = ?, is_archived = ?, updated_at = ?
             WHERE short_id = ?",
            params![title, url, description, category_id, favicon_path, is_archived as i64, now, short_id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound(short_id.into()));
        }
        Ok(())
    }

    /// 物理删除书签（关联表 CASCADE，FTS 手动删）——仅 purge 调用
    pub fn delete_bookmark(&self, short_id: &str) -> CoreResult<()> {
        let conn = self.lock()?;
        let affected = conn.execute("DELETE FROM bookmarks WHERE short_id = ?", params![short_id])?;
        if affected == 0 {
            return Err(CoreError::NotFound(short_id.into()));
        }
        conn.execute("DELETE FROM bookmarks_fts WHERE short_id = ?", params![short_id])?;
        Ok(())
    }

    /// 移入回收站（软删除：打 deleted_at 标记）
    pub fn trash_bookmark(&self, short_id: &str) -> CoreResult<()> {
        let conn = self.lock()?;
        let now = now_iso();
        let affected = conn.execute(
            "UPDATE bookmarks SET deleted_at = ? WHERE short_id = ? AND deleted_at IS NULL",
            params![now, short_id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound(short_id.into()));
        }
        // 从 FTS 移除让搜索不再命中（数据行保留，恢复时由 service 重建 FTS）
        conn.execute("DELETE FROM bookmarks_fts WHERE short_id = ?", params![short_id])?;
        Ok(())
    }

    /// 从回收站恢复（清 deleted_at 标记；FTS 由 service 重建）
    pub fn restore_bookmark(&self, short_id: &str) -> CoreResult<()> {
        let conn = self.lock()?;
        let affected = conn.execute(
            "UPDATE bookmarks SET deleted_at = NULL WHERE short_id = ?",
            params![short_id],
        )?;
        if affected == 0 {
            return Err(CoreError::NotFound(short_id.into()));
        }
        Ok(())
    }

    /// 书签总数（活跃）
    pub fn bookmark_count(&self) -> CoreResult<i64> {
        let conn = self.lock()?;
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM bookmarks WHERE deleted_at IS NULL",
            [],
            |r| r.get(0),
        )?;
        Ok(n)
    }

    /// 书签全文搜索（bm25 排名 + description 列高亮 snippet）。
    /// 仅走 FTS5（英文/前缀可靠）；中文/模糊兜底由 search_all 的 fallback 统一处理（F3 集成）。
    pub fn search_bookmarks(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> CoreResult<Vec<BookmarkSearchHit>> {
        let conn = self.lock()?;
        let fts_query = build_fts_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = conn.prepare(
            "SELECT f.short_id, s.title, s.url,
                    snippet(bookmarks_fts, 3, '<mark>', '</mark>', '…', 16) AS snip,
                    bm25(bookmarks_fts) AS rank
             FROM bookmarks_fts f
             JOIN bookmarks s ON s.short_id = f.short_id
             WHERE bookmarks_fts MATCH ? AND s.deleted_at IS NULL
             ORDER BY rank LIMIT ? OFFSET ?",
        )?;
        let rows = stmt
            .query_map(params![fts_query, limit, offset], |r| {
                Ok(BookmarkSearchHit {
                    short_id: r.get(0)?,
                    title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    url: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    snippet: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    rank: r.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 列出回收站内的书签
    pub fn list_trashed_bookmarks(&self) -> CoreResult<Vec<TrashedItem>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT b.short_id, b.title, b.description, b.deleted_at, b.created_at,
                    (SELECT c.name FROM project_categories c WHERE c.id = b.category_id)
             FROM bookmarks b WHERE b.deleted_at IS NOT NULL
             ORDER BY b.deleted_at DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(TrashedItem {
                    short_id: r.get(0)?,
                    item_type: ItemType::Bookmark,
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

/// 构建 bookmarks 的 WHERE 子句片段与参数（list / count 共用）。
fn bookmark_where(filter: &BookmarkFilter) -> (Vec<String>, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut clauses = vec!["deleted_at IS NULL".to_string()];
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if filter.favorited_only {
        clauses.push("is_favorited = 1".into());
    }
    if let Some(a) = filter.archived {
        clauses.push(format!("is_archived = {}", if a { 1 } else { 0 }));
    }
    if let Some(t) = &filter.tag {
        clauses.push(
            "short_id IN (SELECT bookmark_id FROM bookmark_tag_rel r \
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

/// 从一行装载 Bookmark（不含关联）
fn load_bookmark_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Bookmark> {
    Ok(Bookmark {
        short_id: r.get(0)?,
        title: r.get(1)?,
        url: r.get(2)?,
        description: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
        category_id: r.get(4)?,
        favicon_path: r.get::<_, Option<String>>(5)?.unwrap_or_default(),
        visit_count: r.get(6)?,
        is_archived: r.get::<_, i64>(7)? != 0,
        is_favorited: r.get::<_, i64>(8)? != 0,
        created_at: r.get(9)?,
        updated_at: r.get(10)?,
        tags: Vec::new(),
        category: None,
        deleted_at: None,
    })
}

/// 给 Bookmark 附上标签 + 分类名
fn attach_bookmark_relations(conn: &Connection, b: &mut Bookmark) -> CoreResult<()> {
    let mut stmt = conn.prepare(
        "SELECT t.name FROM bookmark_tag_rel r JOIN tags t ON t.id = r.tag_id
         WHERE r.bookmark_id = ? ORDER BY t.name",
    )?;
    b.tags = stmt
        .query_map(params![b.short_id], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    if let Some(cid) = b.category_id {
        b.category = conn
            .query_row(
                "SELECT name FROM project_categories WHERE id = ?",
                params![cid],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
    }
    Ok(())
}

/// 书签搜索：FTS 命中（含高亮 snippet）+ LIKE 兜底（中文/模糊：title/url/description/tags）。
/// 供 search_all 跨类型统一搜索调用，返回 UnifiedSearchHit（item_type=Bookmark）。
pub(super) fn search_bookmarks_fallback(
    conn: &Connection,
    query: &str,
    like_pat: &str,
    limit: usize,
) -> CoreResult<Vec<UnifiedSearchHit>> {
    let mut out = Vec::new();
    let fts_query = build_fts_query(query);
    if !fts_query.is_empty() {
        let mut stmt = conn.prepare(
            "SELECT f.short_id, s.title, s.url,
                    snippet(bookmarks_fts, 3, '<mark>', '</mark>', '…', 16) AS snip,
                    bm25(bookmarks_fts) AS rank,
                    (SELECT c.name FROM project_categories c WHERE c.id = s.category_id)
             FROM bookmarks_fts f JOIN bookmarks s ON s.short_id = f.short_id
             WHERE bookmarks_fts MATCH ? AND s.deleted_at IS NULL
             ORDER BY rank LIMIT ?",
        )?;
        let rows = stmt
            .query_map(params![fts_query, limit as i64], |r| {
                Ok(UnifiedSearchHit {
                    short_id: r.get(0)?,
                    item_type: ItemType::Bookmark,
                    title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    snippet: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    category: r.get(5)?,
                    rank: r.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
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
            "SELECT s.short_id, s.title, s.url,
                    (SELECT c.name FROM project_categories c WHERE c.id = s.category_id)
             FROM bookmarks s
             WHERE s.deleted_at IS NULL
               AND s.short_id NOT IN ({placeholder})
               AND (s.title LIKE ? OR s.url LIKE ? OR s.description LIKE ?
                    OR s.short_id IN (SELECT r.bookmark_id FROM bookmark_tag_rel r
                                      JOIN tags t ON t.id = r.tag_id WHERE t.name LIKE ?))
             ORDER BY s.created_at DESC LIMIT ?"
        );
        let mut stmt = conn.prepare(&sql)?;
        let remaining = limit - out.len();
        let rows = stmt
            .query_map(params![like_pat, like_pat, like_pat, like_pat, remaining as i64], |r| {
                Ok(UnifiedSearchHit {
                    short_id: r.get(0)?,
                    item_type: ItemType::Bookmark,
                    title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    snippet: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    category: r.get(3)?,
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
