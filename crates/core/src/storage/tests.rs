//! storage 测试（书签单一类型：favicon 缓存）

use super::*;
use std::path::PathBuf;

fn tmp() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "bs-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn write_favicon_persists_and_returns_rel_path() {
    let root = tmp();
    let storage = ProjectStorage::new(root.clone());

    let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let rel = storage.write_favicon("AbCd1234", "png", &png).unwrap();

    // 返回相对路径（正斜杠跨平台一致）
    assert!(rel.starts_with("favicons/"), "相对路径应以 favicons/ 开头：{rel}");
    assert!(rel.ends_with(".png"), "扩展名应为 .png：{rel}");

    // 文件实际落盘到分桶路径
    let abs = storage.resolve_favicon_abs(&rel);
    assert!(abs.exists(), "缓存文件应存在：{}", abs.display());
    assert_eq!(std::fs::read(&abs).unwrap(), png.to_vec());
}

#[test]
fn write_favicon_overwrites_old_extension() {
    let root = tmp();
    let storage = ProjectStorage::new(root.clone());

    // 先写 png，再写 ico：应清理 png，只剩 ico
    storage.write_favicon("IcOe5678", "png", &[1, 2, 3]).unwrap();
    let rel = storage.write_favicon("IcOe5678", "ico", &[4, 5, 6]).unwrap();
    assert!(rel.ends_with(".ico"), "新扩展名应为 .ico：{rel}");

    // 旧 png 应已删除（remove_favicon 在写入前清理全部已知扩展名）
    let png_abs = storage.favicon_file("IcOe5678", "png");
    assert!(!png_abs.exists(), "旧 png 缓存应被清理");
    let ico_abs = storage.favicon_file("IcOe5678", "ico");
    assert!(ico_abs.exists(), "新 ico 缓存应存在");
}

#[test]
fn remove_favicon_deletes_all_extensions() {
    let root = tmp();
    let storage = ProjectStorage::new(root.clone());

    storage.write_favicon("RmV00001", "png", &[1]).unwrap();
    storage.write_favicon("RmV00001", "ico", &[2]).unwrap();
    storage.remove_favicon("RmV00001");

    for ext in ["png", "ico", "svg", "webp", "jpg", "jpeg", "gif"] {
        assert!(
            !storage.favicon_file("RmV00001", ext).exists(),
            "扩展名 {ext} 应已删除"
        );
    }
}

#[test]
fn resolve_favicon_abs_is_root_join() {
    let root = tmp();
    let storage = ProjectStorage::new(root.clone());
    let abs = storage.resolve_favicon_abs("favicons/Ab/Ab/test.png");
    assert_eq!(abs, root.join("favicons/Ab/Ab/test.png"));
}

#[test]
fn favicon_file_is_bucketed_by_prefix() {
    let root = tmp();
    let storage = ProjectStorage::new(root.clone());
    let path = storage.favicon_file("AbCd1234", "png");
    let segs: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let n = segs.len();
    assert!(n >= 4, "路径段数不足: {:?}", segs);
    // 期望尾部四段：favicons / Ab / Cd / AbCd1234.png
    assert_eq!(segs[n - 4], "favicons");
    assert_eq!(segs[n - 3], "Ab", "一级桶应为 shortId 前 2 字符");
    assert_eq!(segs[n - 2], "Cd", "二级桶应为 shortId 第 3-4 字符");
    assert_eq!(segs[n - 1], "AbCd1234.png");
}
