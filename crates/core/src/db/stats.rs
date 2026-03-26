//! 统计类数据库查询（日历热力图等）。

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::CoreResult;

use super::Database;

/// 单日备忘录计数（本地时区日期）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCount {
    /// 本地日期（YYYY-MM-DD）
    pub date: String,
    pub count: i64,
}

impl Database {
    /// 按本地日期聚合未删除备忘录数（日历热力图数据源）
    pub fn list_memo_daily_counts(&self) -> CoreResult<Vec<DailyCount>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT date(created_at, 'localtime') AS d, COUNT(*) AS c
             FROM memo WHERE deleted_at IS NULL
             GROUP BY d ORDER BY d",
        )?;
        let rows = stmt
            .query_map(params![], |r| {
                Ok(DailyCount {
                    date: r.get(0)?,
                    count: r.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }
}
