//! Schema 与迁移
//!
//! 对齐 PRD §5.2 数据模型字段表。迁移采用"用户版本 pragma"策略：
//! 每个版本对应一组 DDL，按序执行。单人本地工具，不做降级。
//!
//! 各版本迁移按 vN 拆到子模块；本模块负责版本管理与按序调度。

use crate::CoreResult;
use rusqlite::{params, Connection};

mod v01_base;
mod v02_03_snippets;
mod v04_notes;
mod v05_06_trash;
mod v07_shares;
mod v08_assets;
mod v09_diagrams;
mod v10_templates;
mod v11_bookmarks;
mod v12_drop_legacy;
mod v13_memos;
mod v14_tag_hierarchy;
mod v15_memo_relations;
mod v16_attachments;
mod v17_category_color;
mod v18_ai_providers;
mod v19_ai_provider_multi;

use v01_base::migrate_v1;
use v02_03_snippets::{migrate_v2, migrate_v3};
use v04_notes::migrate_v4;
use v05_06_trash::{migrate_v5, migrate_v6};
use v07_shares::migrate_v7;
use v08_assets::migrate_v8;
use v09_diagrams::migrate_v9;
use v10_templates::migrate_v10;
use v11_bookmarks::migrate_v11;
use v12_drop_legacy::migrate_v12;
use v13_memos::migrate_v13;
use v14_tag_hierarchy::migrate_v14;
use v15_memo_relations::migrate_v15;
use v16_attachments::migrate_v16;
use v17_category_color::migrate_v17;
use v18_ai_providers::migrate_v18;
use v19_ai_provider_multi::migrate_v19;

/// 当前 schema 版本（每次新增迁移时 +1）
pub const CURRENT_VERSION: u32 = 19;

/// 在全新/既有连接上运行迁移，使其达到 CURRENT_VERSION
pub fn run_migrations(conn: &Connection) -> CoreResult<()> {
    let current = get_user_version(conn)?;
    // 遗留库版本冲突检测必须先于提前返回：byte-link 真实用户库的 user_version
    // 已到 16+，而 byte-memos 转型时复用了 v13+ 编号定义 memo 表族——仅看版本号
    // 会整体跳过 memo 创建，所有 memo 操作报 no such table: memo（P0 #983）。
    // 以「声明已迁移却无 memo 表」为准强制补跑；受影响迁移均带幂等守卫。
    let legacy_collision = current >= 13 && !table_exists(conn, "memo")?;
    if current >= CURRENT_VERSION && !legacy_collision {
        return Ok(());
    }
    if legacy_collision {
        tracing::warn!(
            "检测到遗留库版本冲突（user_version={current} 无 memo 表），补跑 byte-memos v13+ 迁移"
        );
    }
    if current < 1 {
        migrate_v1(conn)?;
    }
    if current < 2 {
        migrate_v2(conn)?;
    }
    if current < 3 {
        migrate_v3(conn)?;
    }
    if current < 4 {
        migrate_v4(conn)?;
    }
    if current < 5 {
        migrate_v5(conn)?;
    }
    if current < 6 {
        migrate_v6(conn)?;
    }
    if current < 7 {
        migrate_v7(conn)?;
    }
    if current < 8 {
        migrate_v8(conn)?;
    }
    if current < 9 {
        migrate_v9(conn)?;
    }
    if current < 10 {
        migrate_v10(conn)?;
    }
    if current < 11 {
        migrate_v11(conn)?;
    }
    if current < 12 {
        migrate_v12(conn)?;
    }
    if current < 13 || legacy_collision {
        migrate_v13(conn)?;
    }
    if current < 14 || legacy_collision {
        migrate_v14(conn)?;
    }
    if current < 15 || legacy_collision {
        migrate_v15(conn)?;
    }
    if current < 16 || legacy_collision {
        migrate_v16(conn)?;
    }
    if current < 17 || legacy_collision {
        migrate_v17(conn)?;
    }
    if current < 18 || legacy_collision {
        migrate_v18(conn)?;
    }
    if current < 19 || legacy_collision {
        migrate_v19(conn)?;
    }

    set_user_version(conn, CURRENT_VERSION)?;
    tracing::info!("数据库迁移完成：{} -> {CURRENT_VERSION}", current);
    Ok(())
}

fn get_user_version(conn: &Connection) -> CoreResult<u32> {
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    Ok(v as u32)
}

/// 表是否存在
fn table_exists(conn: &Connection, name: &str) -> CoreResult<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1",
        params![name],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

/// 列是否存在（幂等 ALTER 守卫用）
fn column_exists(conn: &Connection, table: &str, column: &str) -> CoreResult<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let found = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .any(|c| c == column);
    Ok(found)
}

fn set_user_version(conn: &Connection, version: u32) -> CoreResult<()> {
    conn.execute_batch(&format!("PRAGMA user_version = {version}"))?;
    Ok(())
}

#[cfg(test)]
mod tests;
