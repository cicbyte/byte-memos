//! 业务服务层
//!
//! [`ProjectService`] 是对外的统一 API：持有 [`Database`] 与 [`ProjectStorage`]，
//! 实现笔记 CRUD、标签管理、搜索、统计、分类、回收站等全部业务逻辑。
//! CLI 与桌面端均复用本服务。
//!
//! 各业务域（memos / categories / trash / search / config_backup）按文件拆分，
//! 均以 `impl ProjectService` 扩展方法；顶层路径与配置函数留在本模块，
//! `restore_files` 经 `pub use` 重导出，保持 `crate::service::restore_files` 路径不变。

use std::path::{Path, PathBuf};

use crate::db::{now_iso, Database};
use crate::model::*;
use crate::shortid;
use crate::storage::ProjectStorage;
use crate::{CoreError, CoreResult};

mod categories;
mod config_backup;
mod search;
mod memos;
mod relations;
mod versions;
mod templates;
mod attachments;
mod export;
mod trash;
#[cfg(feature = "ai")]
mod ai;
#[cfg(feature = "ai")]
pub use ai::{
    extract_json_array, extract_json_object, parse_category, parse_tags, AddProviderInput,
    CheckResult, ProviderItem, UpdateProviderInput,
};

// open_at 与顶层配置函数调用的 config_backup 内部 helper（对 service 可见）
use config_backup::{ensure_config, migrate_to_data_layout, read_config, write_config};
// CLI restore 直接调用（crate::service::restore_files）
pub use config_backup::restore_files;
pub use versions::MemoVersionInfo;

/// shortId 碰撞重试上限
const SHORT_ID_MAX_RETRIES: usize = 10;

/// 确定 app 根目录（固定位置，config.toml 在此之下）
///
/// 优先级：`BYTE_MEMOS_HOME` 环境变量 > 默认 `~/.cicbyte/apps/byte-memos/`。
/// 此路径固定不变，config/ 和 data/ 的默认位置都基于它。
pub fn resolve_app_root() -> CoreResult<PathBuf> {
    if let Ok(p) = std::env::var("BYTE_MEMOS_HOME") {
        return Ok(PathBuf::from(p));
    }
    let home = dirs_or_home()?;
    let new_root = home.join(".cicbyte").join("apps").join("byte-memos");
    // 迁移：既有 ~/.cicbyte/apps/byte-link → ~/.cicbyte/apps/byte-memos
    let old_root = home.join(".cicbyte").join("apps").join("byte-link");
    if old_root.exists() && !new_root.exists() {
        if let Some(parent) = new_root.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::rename(&old_root, &new_root) {
            Ok(()) => tracing::info!("已迁移应用目录 {} → {}", old_root.display(), new_root.display()),
            Err(e) => tracing::warn!("迁移应用目录失败：{e}（继续使用新路径）"),
        }
    }
    Ok(new_root)
}

/// 确定实际数据目录（可由用户在设置页自定义）
///
/// 优先级：config.toml 的 data_dir 字段 > 默认 app_root/data/
/// config.toml 永远在 app_root/config/ 下（固定锚点），data 可移动到任意位置。
pub fn resolve_data_dir() -> CoreResult<PathBuf> {
    let app_root = resolve_app_root()?;
    let cfg = read_config(&app_root).unwrap_or_default();
    if let Some(d) = &cfg.data_dir {
        if !d.is_empty() {
            let path = PathBuf::from(d);
            if path.is_dir() {
                return Ok(path);
            }
            tracing::warn!("config.toml 的 data_dir 指向的目录不存在，回退到默认：{}", path.display());
        }
    }
    Ok(app_root.join("data"))
}

/// 设置自定义数据目录（写入 config.toml 的 data_dir 字段，重启后生效）
pub fn set_data_dir(dir: &Path) -> CoreResult<()> {
    let app_root = resolve_app_root()?;
    let mut cfg = read_config(&app_root).unwrap_or_default();
    cfg.data_dir = Some(dir.to_string_lossy().into_owned());
    write_config(&app_root, &cfg)?;
    tracing::info!("数据目录已设置为：{}", dir.display());
    Ok(())
}

/// 清除自定义数据目录（恢复默认 app_root/data/，重启后生效）
pub fn clear_data_dir() -> CoreResult<()> {
    let app_root = resolve_app_root()?;
    let mut cfg = read_config(&app_root).unwrap_or_default();
    cfg.data_dir = None;
    write_config(&app_root, &cfg)?;
    tracing::info!("数据目录已恢复默认");
    Ok(())
}

/// 设置导入时排除的目录名列表（写入 config.toml 的 [preferences].exclude_dirs）
///
/// 即时生效：下次 `add()` / `import_entry()` 会现读此配置。传空 Vec 表示不排除任何目录。
pub fn set_exclude_dirs(dirs: Vec<String>) -> CoreResult<()> {
    let app_root = resolve_app_root()?;
    let mut cfg = read_config(&app_root).unwrap_or_default();
    cfg.preferences.exclude_dirs = dirs;
    write_config(&app_root, &cfg)?;
    tracing::info!("排除目录偏好已更新");
    Ok(())
}

fn dirs_or_home() -> CoreResult<PathBuf> {
    if let Some(h) = home_dir_via_env() {
        return Ok(h);
    }
    Err(CoreError::Other(anyhow::anyhow!(
        "无法定位用户家目录（HOME / USERPROFILE 均未设置）"
    )))
}

fn home_dir_via_env() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// 核心业务服务
pub struct ProjectService {
    pub db: Database,
    pub storage: ProjectStorage,
    /// 数据根目录（~/.cicbyte/apps/byte-memos/），data/ 和 config/ 在此之下
    data_root: PathBuf,
}

impl ProjectService {
    /// 在默认位置打开服务（app_root 固定，data_dir 由 config.toml 决定）
    pub fn open_default() -> CoreResult<Self> {
        let app_root = resolve_app_root()?;
        let data_dir = resolve_data_dir()?;
        Self::open_at(&app_root, &data_dir)
    }

    /// 打开服务（app_root 放 config，data_dir 放数据）
    ///
    /// app_root 固定不变（config/ 在此），data_dir 可为任意路径（默认 app_root/data/）。
    pub fn open_at<P: AsRef<Path>, Q: AsRef<Path>>(app_root: P, data_dir: Q) -> CoreResult<Self> {
        let app_root = app_root.as_ref().to_path_buf();
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&app_root)?;
        std::fs::create_dir_all(&data_dir)?;
        // 迁移旧布局（仅当 data_dir == app_root，即旧版数据散在根目录的场景）
        if data_dir == app_root {
            migrate_to_data_layout(&app_root)?;
        }
        let db_dir = data_dir.join("db");
        std::fs::create_dir_all(&db_dir)?;
        // 迁移：既有 byte-link.db → byte-memos.db（品牌统一，含 WAL/SHM 辅助文件）
        let old_db = db_dir.join("byte-link.db");
        let new_db = db_dir.join("byte-memos.db");
        if old_db.exists() && !new_db.exists() {
            if let Err(e) = std::fs::rename(&old_db, &new_db) {
                tracing::warn!("迁移 byte-link.db → byte-memos.db 失败：{e}");
            } else {
                for ext in ["-wal", "-shm"] {
                    let ow = db_dir.join(format!("byte-link.db{ext}"));
                    let nw = db_dir.join(format!("byte-memos.db{ext}"));
                    if ow.exists() {
                        let _ = std::fs::rename(&ow, &nw);
                    }
                }
                tracing::info!("已迁移数据库文件 byte-link.db → byte-memos.db");
            }
        }
        // 优先 byte-memos.db；仅当迁移失败时回退 byte-link.db；
        // 全新库用 new_db 创建 byte-memos.db
        let db_path = if old_db.exists() && !new_db.exists() {
            old_db
        } else {
            new_db
        };
        let db = Database::open(&db_path)?;
        let storage = ProjectStorage::new(data_dir);
        let svc = Self { db, storage, data_root: app_root.clone() };
        // 首次启动写默认 config.toml
        ensure_config(&app_root)?;
        Ok(svc)
    }

    /// 数据根目录（~/.cicbyte/apps/byte-memos/，config/ 和 data/ 在此之下）
    pub fn data_root(&self) -> &Path {
        &self.data_root
    }

    /// 数据目录（data/，内容文件与 db 在此之下）
    pub fn data_dir(&self) -> &Path {
        self.storage.root()
    }
}

#[cfg(test)]
mod tests;
