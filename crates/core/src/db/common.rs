//! 跨实体共享的数据库辅助函数（WHERE 构建器、批量关联加载、时间戳、FTS 查询构造）。
//!
//! 全部为 `pub(super)`：仅 db 模块内部（含各实体子模块与测试）可见；
//! `now_iso` 经 mod.rs `pub use` 重导出供 service 层使用。

use rusqlite::{params, Connection};

use crate::CoreResult;

/// 取得或创建「按名唯一」记录的 id
pub(super) fn upsert_named(
    conn: &Connection,
    insert_sql: &str,
    table: &str,
    name: &str,
) -> CoreResult<i64> {
    conn.execute(insert_sql, params![name])?;
    let id: i64 = conn.query_row(
        &format!("SELECT id FROM {table} WHERE name = ?"),
        params![name],
        |r| r.get(0),
    )?;
    Ok(id)
}

/// 把 WHERE 片段拼成 `WHERE ... AND ...`（clauses 恒非空，至少有 deleted_at IS NULL）。
pub(super) fn join_where(clauses: &[String]) -> String {
    if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    }
}

/// 批量加载多对多关联名（tags / languages），消灭列表查询的 N+1。
///
/// 一次 `IN (...)` 查询取回本页所有 owner 的关联名，按 owner(short_id) 分组，
/// 组内按名升序（与原 `attach_*_relations` 的 `ORDER BY t.name` 一致）。
///
/// 表/列名为代码常量拼接（非用户输入），无注入风险；owner id 走绑定参数。
pub(super) fn load_rel_names_batch(
    conn: &Connection,
    rel_table: &str,
    owner_col: &str,
    name_table: &str,
    link_col: &str,
    ids: &[String],
) -> CoreResult<std::collections::HashMap<String, Vec<String>>> {
    let mut map = std::collections::HashMap::new();
    if ids.is_empty() {
        return Ok(map);
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT r.{owner_col}, n.name FROM {rel_table} r
         JOIN {name_table} n ON n.id = r.{link_col}
         WHERE r.{owner_col} IN ({placeholders}) ORDER BY n.name"
    );
    let params: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let mut stmt = conn.prepare(&sql)?;
    for row in stmt
        .query_map(params.as_slice(), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
    {
        let (owner, name) = row?;
        map.entry(owner).or_insert_with(Vec::new).push(name);
    }
    Ok(map)
}

/// 批量加载分类名：`SELECT id, name FROM project_categories WHERE id IN (...)`
pub(super) fn load_category_names_batch(
    conn: &Connection,
    cids: &[i64],
) -> CoreResult<std::collections::HashMap<i64, String>> {
    let mut map = std::collections::HashMap::new();
    if cids.is_empty() {
        return Ok(map);
    }
    let placeholders = cids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("SELECT id, name FROM project_categories WHERE id IN ({placeholders})");
    let params: Vec<&dyn rusqlite::ToSql> = cids.iter().map(|c| c as &dyn rusqlite::ToSql).collect();
    let mut stmt = conn.prepare(&sql)?;
    for row in stmt.query_map(params.as_slice(), |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })? {
        let (id, name) = row?;
        map.insert(id, name);
    }
    Ok(map)
}

/// 当前 ISO 8601 时间戳（UTC）
pub fn now_iso() -> String {
    use chrono::Utc;
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// 把用户查询构造为 FTS5 查询表达式：按空白切分，每段加前缀匹配 *
/// 例 "mcp server" -> "mcp* server*"
pub(super) fn build_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|t| {
            // 转义双引号，整体用短语包裹以支持含特殊字符的 token
            let escaped = t.replace('"', "\"\"");
            format!("\"{escaped}\"*")
        })
        .collect::<Vec<_>>()
        .join(" ")
}
