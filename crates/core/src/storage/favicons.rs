//! favicon 本地缓存（F11）：抓取的站点图标按 shortId 分桶落盘到 `data/favicons/`。
//!
//! 路径约定：`favicons/{前2字符}/{第3-4字符}/{shortId}.{ext}`，与 snippets/notes/templates
//! 分桶一致，避免单目录万级文件平铺。入库的 `favicon_path` 为**相对 data_dir 的路径**
//! （迁移数据目录时仍有效；Desktop 用 `convertFileSrc` 把绝对路径转成 webview 可访问 URL）。

use super::helpers::bucket_path;
use super::ProjectStorage;
use crate::{CoreError, CoreResult};

/// 已知 favicon 扩展名（清理旧缓存时逐个尝试删除）
const FAVICON_EXTS: &[&str] = &["png", "ico", "svg", "webp", "jpg", "jpeg", "gif"];

impl ProjectStorage {
    /// favicons 根目录：`data/favicons/`
    pub fn favicons_dir(&self) -> std::path::PathBuf {
        self.root.join("favicons")
    }

    /// 某书签 favicon 的分桶文件路径：`favicons/{前2}/{第3-4}/{shortId}.{ext}`
    pub fn favicon_file(&self, short_id: &str, ext: &str) -> std::path::PathBuf {
        bucket_path(&self.favicons_dir(), short_id, &format!("{short_id}.{ext}"))
    }

    /// 写 favicon 字节到分桶路径，返回相对 data_dir 的路径（入库用，正斜杠跨平台一致）。
    /// 写入前先清理该 shortId 可能存在的旧扩展名缓存，避免残留多份。
    pub fn write_favicon(&self, short_id: &str, ext: &str, bytes: &[u8]) -> CoreResult<String> {
        self.remove_favicon(short_id);
        let path = self.favicon_file(short_id, ext);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
        let rel = path
            .strip_prefix(&self.root)
            .map_err(|e| CoreError::Other(anyhow::anyhow!("favicon 路径剥离失败：{e}")))?;
        Ok(rel.to_string_lossy().replace('\\', "/"))
    }

    /// 删除某书签的全部 favicon 缓存（遍历已知扩展名，存在则删；失败忽略）。
    pub fn remove_favicon(&self, short_id: &str) {
        for ext in FAVICON_EXTS {
            let p = self.favicon_file(short_id, ext);
            if p.exists() {
                let _ = std::fs::remove_file(p);
            }
        }
    }

    /// 由入库的相对路径还原绝对路径（Desktop convertFileSrc 用）。
    pub fn resolve_favicon_abs(&self, rel_path: &str) -> std::path::PathBuf {
        self.root.join(rel_path)
    }
}
