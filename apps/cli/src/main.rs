//! byte-memos CLI 入口
//!
//! 子命令式：`bm <command>`。骨架阶段仅定义命令树与版本/帮助，
//! 各子命令的业务逻辑在后续功能填充。

mod cmd;
mod format;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "bm",
    version,
    about = "byte-memos — 本地备忘录管理 + MCP/REST 集成",
    long_about = "纯本地、单人、零部署的备忘录管理应用。\n统一管理备忘录：增删改查、标签分类、全文搜索、置顶、可见性管理。\n通过 CLI / Desktop / HTTP / MCP 多端操作同一份本地 SQLite。",
    propagate_version = true
)]
struct Cli {
    /// 输出 JSON（脚本/AI 友好）而非彩色人类可读输出
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 备忘录管理（byte-memos 核心领域）
    #[command(subcommand)]
    Memo(cmd::MemoCommand),
    /// 分类管理
    #[command(subcommand)]
    Category(cmd::CategoryCommand),
    /// 标签管理
    #[command(subcommand)]
    Tag(cmd::TagCommand),
    /// 模板管理
    #[command(subcommand)]
    Template(cmd::TemplateCommand),
    /// 导出备份（zip 或目录快照）
    Backup(cmd::BackupArgs),
    /// 从备份恢复（覆盖当前数据）
    Restore(cmd::RestoreArgs),
    /// 运行 MCP 服务器（供 Claude Desktop / Cursor 等 AI 工具检索）
    #[command(name = "mcp")]
    Mcp(cmd::McpArgs),
    /// 运行 REST API 服务器（供第三方应用 HTTP 对接）
    #[command(subcommand)]
    Api(cmd::ApiAction),
}

fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();

    // 骨架阶段：未传子命令时打印帮助
    let Some(command) = cli.command else {
        println!("byte-memos {}\n", env!("CARGO_PKG_VERSION"));
        println!("用 `bm --help` 查看可用命令。");
        return Ok(());
    };

    // 按需启用详细实现。骨架阶段先 wire 一个友好的提示。
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async move {
        match command {
            Commands::Memo(sub) => match sub {
                cmd::MemoCommand::Add(a) => cmd::run_memo_add(a, cli.json).await,
                cmd::MemoCommand::List(a) => cmd::run_memo_list(a, cli.json).await,
                cmd::MemoCommand::Get(a) => cmd::run_memo_get(a, cli.json).await,
                cmd::MemoCommand::Update(a) => cmd::run_memo_update(a, cli.json).await,
                cmd::MemoCommand::Delete(a) => cmd::run_memo_delete(a, cli.json).await,
                cmd::MemoCommand::Search(a) => cmd::run_memo_search(a, cli.json).await,
                cmd::MemoCommand::Tag(a) => cmd::run_memo_tag(a, cli.json).await,
                cmd::MemoCommand::Pin(a) => cmd::run_memo_pin(a, cli.json).await,
                cmd::MemoCommand::Visibility(a) => cmd::run_memo_visibility(a, cli.json).await,
                cmd::MemoCommand::Link(a) => cmd::run_memo_link(a, cli.json).await,
                cmd::MemoCommand::Unlink(a) => cmd::run_memo_unlink(a, cli.json).await,
                cmd::MemoCommand::Relations(a) => cmd::run_memo_relations(a, cli.json).await,
                cmd::MemoCommand::History(a) => cmd::run_memo_history(a, cli.json).await,
                cmd::MemoCommand::RestoreVersion(a) => cmd::run_memo_restore_version(a, cli.json).await,
            },
            Commands::Category(sub) => match sub {
                cmd::CategoryCommand::List => cmd::run_category_list(cli.json).await,
                cmd::CategoryCommand::Create(a) => cmd::run_category_create(a, cli.json).await,
                cmd::CategoryCommand::Rename(a) => cmd::run_category_rename(a, cli.json).await,
                cmd::CategoryCommand::Delete(a) => cmd::run_category_delete(a, cli.json).await,
            },
            Commands::Tag(sub) => match sub {
                cmd::TagCommand::List => cmd::run_tag_list(cli.json).await,
                cmd::TagCommand::SetColor { name, color } => {
                    cmd::run_tag_set_color(cli.json, name, color).await
                }
                cmd::TagCommand::Delete { name } => cmd::run_tag_delete(cli.json, name).await,
            },
            Commands::Template(sub) => match sub {
                cmd::TemplateCommand::List => cmd::run_template_list(cli.json).await,
                cmd::TemplateCommand::Create(a) => cmd::run_template_create(cli.json, a).await,
                cmd::TemplateCommand::Delete(a) => cmd::run_template_delete(cli.json, a).await,
                cmd::TemplateCommand::Use(a) => cmd::run_template_use(cli.json, a).await,
                cmd::TemplateCommand::SaveAs(a) => cmd::run_template_save_as(cli.json, a).await,
            },
            Commands::Backup(a) => cmd::run_backup(a, cli.json).await,
            Commands::Restore(a) => cmd::run_restore(a, cli.json).await,
            Commands::Mcp(a) => cmd::run_mcp(a).await,
            Commands::Api(a) => cmd::run_api(cmd::ApiArgs { action: a }).await,
        }
    })?;
    Ok(())
}

/// 初始化日志：默认 INFO 级别，写到 stderr（绝不污染 stdout，MCP stdio 模式依赖此约定）。
fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();
}
