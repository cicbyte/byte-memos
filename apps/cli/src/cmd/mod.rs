//! CLI 子命令定义与实现，按业务域拆分到各子模块
//!
//! 通过 `pub use` 把各域的 Args 结构 / `run_*` 函数重导出到 `cmd::` 名字空间，
//! 使 `main.rs` 的引用路径（`cmd::MemoCommand`、`cmd::run_memo_add` 等）保持不变。

mod memos;
mod backup;
mod category;
mod server;
mod tags;
mod templates;

// ---- 备份 / 恢复 ----
pub use backup::{BackupArgs, RestoreArgs, run_backup, run_restore};

// ---- memo 子命令（byte-memos 核心领域）----
pub use memos::{
    MemoCommand, run_memo_add, run_memo_delete, run_memo_get, run_memo_history,
    run_memo_link, run_memo_list, run_memo_pin, run_memo_relations,
    run_memo_restore_version, run_memo_search, run_memo_tag, run_memo_unlink,
    run_memo_update, run_memo_visibility,
};

// ---- category 子命令 ----
pub use category::{
    CategoryCommand, run_category_create, run_category_delete, run_category_list,
    run_category_rename,
};

// ---- tag 子命令 ----
pub use tags::{TagCommand, run_tag_list, run_tag_set_color, run_tag_delete};

// ---- template 子命令 ----
pub use templates::{
    TemplateCommand, run_template_create, run_template_delete, run_template_list,
    run_template_save_as, run_template_use,
};

// ---- MCP / REST API 服务 ----
pub use server::{ApiAction, ApiArgs, McpArgs, run_api, run_mcp};

// ===== 公共辅助（供各域 run_* 复用）=====

/// 打开 service；失败时返回 anyhow 错误（json 参数保留签名兼容，供各 run_* 传递）
pub(crate) fn open_service(_json: bool) -> anyhow::Result<byte_memos_core::ProjectService> {
    byte_memos_core::ProjectService::open_default()
        .map_err(|e| anyhow::anyhow!("初始化数据目录失败：{e}"))
}
