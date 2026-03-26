//! 书签（bookmarks）：CRUD、列表、收藏、归档、访问计数、回收站。
//!
//! 与 snippets 的差异：description 直接存 DB（不落盘），无 storage 读写，
//! FTS 由 db 层 `sync_bookmark_fts` 直接同步 DB 列。url 去重在活跃行内生效。

use super::*;

impl super::ProjectService {
    // ============ 书签 ============

    /// 新增书签：url 校验 → 生成 shortId → url 去重 → 写库 → 关联标签 → 同步 FTS
    pub fn add_bookmark(&self, input: &AddBookmarkInput) -> CoreResult<Bookmark> {
        if input.url.trim().is_empty() {
            return Err(CoreError::Other(anyhow::anyhow!("书签 url 不能为空")));
        }
        // 活跃行内 url 去重
        if let Some(existing) = self.db.get_bookmark_by_url(&input.url)? {
            return Err(CoreError::AlreadyExists(format!(
                "书签 url 已存在：{}（id={}）",
                existing.url, existing.short_id
            )));
        }
        let short_id = self.generate_unique_bookmark_id()?;
        let category_id = match &input.category {
            Some(c) => Some(self.db.upsert_category(c)?),
            None => None,
        };
        let now = now_iso();
        let bm = Bookmark {
            short_id: short_id.clone(),
            title: if input.title.trim().is_empty() {
                input.url.clone()
            } else {
                input.title.clone()
            },
            url: input.url.clone(),
            description: input.description.clone().unwrap_or_default(),
            category_id,
            category: input.category.clone(),
            favicon_path: input.favicon_path.clone().unwrap_or_default(),
            visit_count: 0,
            is_archived: false,
            is_favorited: false,
            created_at: now.clone(),
            updated_at: now,
            tags: input.tags.clone(),
            deleted_at: None,
        };
        self.db.insert_bookmark(&bm)?;
        let tag_ids: Vec<i64> = input
            .tags
            .iter()
            .map(|t| self.db.upsert_tag(t))
            .collect::<CoreResult<Vec<_>>>()?;
        self.db.set_bookmark_tags(&short_id, &tag_ids)?;
        self.db.sync_bookmark_fts(&short_id)?;
        self.db
            .get_bookmark(&short_id)?
            .ok_or(CoreError::NotFound(short_id))
    }

    fn generate_unique_bookmark_id(&self) -> CoreResult<String> {
        for _ in 0..SHORT_ID_MAX_RETRIES {
            let id = shortid::generate();
            if !self.db.bookmark_id_exists(&id)? {
                return Ok(id);
            }
        }
        Err(CoreError::ShortIdCollision(SHORT_ID_MAX_RETRIES))
    }

    /// 按 shortId 取书签（查看详情，不自增 visit；open 才自增）
    pub fn get_bookmark(&self, short_id: &str) -> CoreResult<Bookmark> {
        self.db
            .get_bookmark(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))
    }

    /// 按 url 取书签
    pub fn get_bookmark_by_url(&self, url: &str) -> CoreResult<Option<Bookmark>> {
        self.db.get_bookmark_by_url(url)
    }

    pub fn list_bookmarks(&self, filter: &BookmarkFilter) -> CoreResult<Vec<Bookmark>> {
        self.db.list_bookmarks(filter)
    }

    /// 分页列出书签（items + total）
    pub fn list_bookmarks_paged(&self, filter: &BookmarkFilter) -> CoreResult<PagedResult<Bookmark>> {
        let items = self.db.list_bookmarks(filter)?;
        let total = self.db.count_bookmarks(filter)?;
        Ok(PagedResult { items, total })
    }

    /// 更新书签（覆盖式）：url 变更时去重（排除自身）+ 元数据 + 标签重置 + FTS 刷新
    pub fn update_bookmark(&self, short_id: &str, input: &UpdateBookmarkInput) -> CoreResult<Bookmark> {
        let old = self
            .db
            .get_bookmark(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))?;
        // url 变更时去重（排除自身）
        if input.url != old.url {
            if let Some(other) = self.db.get_bookmark_by_url(&input.url)? {
                if other.short_id != short_id {
                    return Err(CoreError::AlreadyExists(format!(
                        "书签 url 已存在：{}",
                        input.url
                    )));
                }
            }
        }
        let category_id = match &input.category {
            Some(c) => Some(self.db.upsert_category(c)?),
            None => None,
        };
        let favicon_path = input
            .favicon_path
            .clone()
            .unwrap_or_else(|| old.favicon_path.clone());
        let is_archived = input.is_archived.unwrap_or(old.is_archived);
        let title = if input.title.trim().is_empty() {
            input.url.clone()
        } else {
            input.title.clone()
        };
        self.db.update_bookmark_meta(
            short_id,
            &title,
            &input.url,
            &input.description.clone().unwrap_or_default(),
            category_id,
            &favicon_path,
            is_archived,
        )?;
        let tag_ids: Vec<i64> = input
            .tags
            .iter()
            .map(|t| self.db.upsert_tag(t))
            .collect::<CoreResult<Vec<_>>>()?;
        self.db.set_bookmark_tags(short_id, &tag_ids)?;
        self.db.sync_bookmark_fts(short_id)?;
        self.db
            .get_bookmark(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))
    }

    /// 记录一次访问（「打开浏览器」时由 transport 层调用）
    pub fn increment_bookmark_visit(&self, short_id: &str) -> CoreResult<()> {
        self.db.increment_bookmark_visit(short_id)
    }

    /// 移入回收站（软删除：打标记，标签关联保留）
    pub fn delete_bookmark(&self, short_id: &str) -> CoreResult<()> {
        self.db.trash_bookmark(short_id)?;
        Ok(())
    }

    /// 从回收站恢复（清标记 + 重建 FTS）
    pub fn restore_bookmark(&self, short_id: &str) -> CoreResult<()> {
        self.db.restore_bookmark(short_id)?;
        self.db.sync_bookmark_fts(short_id)?;
        Ok(())
    }

    /// 彻底删除书签（物理删除 DB 行 + 关联 CASCADE + FTS）
    /// favicon 缓存文件清理留待 M5（有 favicon 时）
    pub fn purge_bookmark(&self, short_id: &str) -> CoreResult<()> {
        self.db.delete_bookmark(short_id)?;
        Ok(())
    }

    pub fn toggle_bookmark_favorite(&self, short_id: &str) -> CoreResult<Bookmark> {
        let cur = self
            .db
            .get_bookmark(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))?;
        self.db.set_bookmark_favorited(short_id, !cur.is_favorited)?;
        self.db
            .get_bookmark(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))
    }

    /// 设置归档状态
    pub fn set_bookmark_archived(&self, short_id: &str, archived: bool) -> CoreResult<Bookmark> {
        self.db.set_bookmark_archived(short_id, archived)?;
        self.db
            .get_bookmark(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))
    }

    /// 活跃书签总数
    pub fn bookmark_count(&self) -> CoreResult<i64> {
        self.db.bookmark_count()
    }

    /// 书签全文搜索（FTS5 bm25 排名 + 高亮 snippet）
    pub fn search_bookmarks(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> CoreResult<Vec<BookmarkSearchHit>> {
        self.db.search_bookmarks(query, limit, offset)
    }

    // ============ favicon 本地缓存（F11） ============

    /// 把已下载的 favicon 字节落盘并入库（返回相对 data_dir 的路径）。
    /// 供 fetch 成功后调用；亦单独暴露以便测试（无需网络）。
    pub fn save_favicon(
        &self,
        short_id: &str,
        bytes: &[u8],
        content_type: &str,
    ) -> CoreResult<String> {
        let old = self
            .db
            .get_bookmark(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))?;
        let ext = favicon_ext(content_type);
        let rel = self.storage.write_favicon(short_id, ext, bytes)?;
        // 其余字段保留原值，仅更新 favicon_path
        self.db.update_bookmark_meta(
            short_id,
            &old.title,
            &old.url,
            &old.description,
            old.category_id,
            &rel,
            old.is_archived,
        )?;
        Ok(rel)
    }

    /// 抓取并缓存某书签的 favicon（best-effort：失败返回 `Ok(None)`，不报错、不阻断业务）。
    ///
    /// **设计**：不在 `add_bookmark` 内自动调用——由 CLI/Desktop 在 add 后显式触发，
    /// 避免 core 测试依赖网络、保持 add 路径纯净稳定。
    pub fn fetch_favicon_for_bookmark(&self, short_id: &str) -> CoreResult<Option<String>> {
        let bm = self
            .db
            .get_bookmark(short_id)?
            .ok_or_else(|| CoreError::NotFound(short_id.into()))?;
        match download_favicon(&bm.url) {
            Some((bytes, ct)) => match self.save_favicon(short_id, &bytes, &ct) {
                Ok(rel) => Ok(Some(rel)),
                Err(e) => {
                    tracing::warn!("保存 favicon 失败 {short_id}：{e}");
                    Ok(None)
                }
            },
            None => Ok(None),
        }
    }
}

// ============ favicon 抓取辅助（best-effort，纯函数式，失败一律返回 None）============

/// 从 url 抓取 favicon 字节 + content-type。
///
/// 策略：① GET 站点首页 HTML，正则找 `<link rel="icon"|"shortcut icon" href>`，
///         解析其图标 url 后下载；② 退化到 `{scheme}://{host}/favicon.ico`。
/// 约束：整体超时 6s、连接超时 4s、仅接受 `image/*`、体积 ≤ 256KB、最多 5 次重定向。
fn download_favicon(url: &str) -> Option<(Vec<u8>, String)> {
    // reqwest::blocking 在 tokio runtime context 内会 panic（"Cannot start a runtime from
    // within a runtime"）；CLI/Desktop 的 async 命令链运行在 tokio 上，故隔离到独立线程
    // （无 runtime context）执行。core 单元测试无 tokio runtime，统一走此路径保持一致。
    let url = url.to_string();
    std::thread::spawn(move || download_favicon_blocking(&url))
        .join()
        .ok()
        .flatten()
}

fn download_favicon_blocking(url: &str) -> Option<(Vec<u8>, String)> {
    use std::time::Duration;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(6))
        .connect_timeout(Duration::from_secs(4))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("byte-link/1.0")
        .no_proxy() // favicon 直连：忽略系统代理（best-effort，代理网络下降级为空，不阻断业务）
        .build()
        .ok()?;
    let base = reqwest::Url::parse(url).ok()?;
    let scheme = base.scheme();
    let host = base.host_str()?;
    // 站点根（含端口）：host_str() 不含端口，非默认端口须显式拼上，否则本地/非标准端口的站点连不上
    let host_root = match base.port() {
        Some(p) => format!("{scheme}://{host}:{p}"),
        None => format!("{scheme}://{host}"),
    };

    // ① 解析首页 HTML 找 <link rel=icon>
    let html_resp = client
        .get(&host_root)
        .header("Accept", "text/html,*/*;q=0.8")
        .send();
    if let Ok(resp) = html_resp {
        if resp.status().is_success() {
            if let Ok(html) = resp.text() {
                if let Some(icon_url) = pick_icon_href(&html) {
                    let abs = base
                        .join(&icon_url)
                        .ok()
                        .map(|u| u.to_string())
                        .unwrap_or(icon_url);
                    if let Some(got) = fetch_image(&client, &abs) {
                        return Some(got);
                    }
                }
            }
        }
    }
    // ② 退化 favicon.ico
    fetch_image(&client, &format!("{host_root}/favicon.ico"))
}

/// 从首页 HTML 提取第一个 favicon `<link>` 的 href（大小写不敏感）。
fn pick_icon_href(html: &str) -> Option<String> {
    use regex::Regex;
    let link_re =
        Regex::new(r#"(?i)<link[^>]+rel\s*=\s*["']?(?:shortcut\s+icon|icon)["']?[^>]*>"#).ok()?;
    let href_re = Regex::new(r#"(?i)href\s*=\s*["']([^"']+)["']"#).ok()?;
    let link_tag = link_re.find(html)?;
    let caps = href_re.captures(link_tag.as_str())?;
    let href = caps.get(1)?.as_str().trim();
    if href.is_empty() {
        None
    } else {
        Some(href.to_string())
    }
}

/// 下载单个图片 URL，校验体积（≤256KB），返回 (bytes, content_type)。
///
/// content-type 判定放宽：`image/*` 直接接受；否则按 URL 扩展名兜底（兼容返回
/// `application/octet-stream` 的 `.ico` 等不规范响应——Windows Python http.server 即如此）。
fn fetch_image(client: &reqwest::blocking::Client, url: &str) -> Option<(Vec<u8>, String)> {
    let resp = client
        .get(url)
        .header("Accept", "image/*,*/*;q=0.8")
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    // content-type 缺失时按 octet-stream 兜底（之后用 URL 扩展名补救）
    let ct_raw = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_ascii_lowercase();
    let ct_main = ct_raw
        .split(';')
        .next()
        .unwrap_or(&ct_raw)
        .trim()
        .to_string();
    // 体积上限 256KB：content-length 头 + 实际字节双重校验
    if resp.content_length().unwrap_or(0) > 256 * 1024 {
        return None;
    }
    let bytes = resp.bytes().ok()?;
    if bytes.len() > 256 * 1024 {
        return None;
    }
    if ct_main.starts_with("image/") {
        Some((bytes.to_vec(), ct_main))
    } else {
        url_ext(url).map(|e| (bytes.to_vec(), ext_to_ct(e).to_string()))
    }
}

/// 从 URL 路径提取已知图标扩展名（小写）；非图标扩展返回 None。
fn url_ext(url: &str) -> Option<&'static str> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let last = parsed.path().rsplit('/').next()?;
    let dot = last.rfind('.')?;
    let ext = last[dot + 1..].to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => "png",
        "ico" => "ico",
        "svg" => "svg",
        "webp" => "webp",
        "jpg" | "jpeg" => "jpg",
        "gif" => "gif",
        _ => return None,
    })
}

/// 扩展名 → content-type（URL 兜底路径用，与 [`favicon_ext`] 互逆）
fn ext_to_ct(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        _ => "image/png",
    }
}

/// content-type → favicon 文件扩展名（未知类型默认 png）。
fn favicon_ext(content_type: &str) -> &'static str {
    match content_type {
        "image/png" => "png",
        "image/x-icon" | "image/vnd.microsoft.icon" => "ico",
        "image/svg+xml" => "svg",
        "image/webp" => "webp",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        _ => "png",
    }
}
