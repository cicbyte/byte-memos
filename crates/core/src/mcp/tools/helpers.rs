//! MCP 工具辅助函数：mark 剥离、大小格式化

pub(super) fn strip_marks(s: &str) -> String {
    s.replace("<mark>", "").replace("</mark>", "")
}

pub(super) fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}
