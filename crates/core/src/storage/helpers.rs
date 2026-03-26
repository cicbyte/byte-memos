//! 存储辅助函数：分桶路径
//!
//! 知识库文件树构造 / 资产缩略图 / 目录递归拷贝等辅助已随知识库移除，
//! 仅保留书签 favicon 缓存用的分桶路径。

use std::path::{Path, PathBuf};

/// 基于前缀分桶的文件路径：`{base}/{前2字符}/{第3-4字符}/{filename}`
///
/// shortId 8 字符取前 2 做一级目录、第 3-4 做二级目录，避免单目录万级文件平铺。
/// shortId 不足 4 字符时退化为按实际长度分桶（不会发生——shortId 固定 8 字符）。
pub(super) fn bucket_path(base: &Path, short_id: &str, filename: &str) -> PathBuf {
    let chars: Vec<char> = short_id.chars().collect();
    let (d1, d2) = if chars.len() >= 4 {
        (
            chars[..2].iter().collect::<String>(),
            chars[2..4].iter().collect::<String>(),
        )
    } else if chars.len() >= 2 {
        (chars[..2].iter().collect::<String>(), "_".into())
    } else {
        ("_".into(), "_".into())
    };
    base.join(d1).join(d2).join(filename)
}
