//! category 子命令：列表 / 创建 / 重命名 / 删除

use clap::{Args, Subcommand};

use super::open_service;
use crate::format;

#[derive(Subcommand, Debug)]
pub enum CategoryCommand {
    /// 列出全部分类
    List,
    /// 创建分类
    Create(CategoryCreateArgs),
    /// 重命名分类
    Rename(CategoryRenameArgs),
    /// 删除分类（其下内容保留，仅解除绑定）
    Delete(CategoryDeleteArgs),
}

#[derive(Args, Debug)]
pub struct CategoryCreateArgs {
    /// 分类名
    pub name: String,
}

#[derive(Args, Debug)]
pub struct CategoryRenameArgs {
    /// 分类 ID
    #[arg(long)]
    pub id: i64,
    /// 新名称
    #[arg(long)]
    pub name: String,
}

#[derive(Args, Debug)]
pub struct CategoryDeleteArgs {
    /// 分类 ID
    #[arg(long)]
    pub id: i64,
    /// 跳过确认
    #[arg(long, short = 'y')]
    pub yes: bool,
}

pub async fn run_category_list(json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let cats = svc.list_categories()?;
    format::print_categories(&cats, json);
    Ok(())
}

pub async fn run_category_create(args: CategoryCreateArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let id = svc.create_category(&args.name)?;
    if json {
        println!("{{\"id\":{id},\"name\":\"{}\"}}", args.name);
    } else {
        println!("✅ 已创建分类「{}」(id={id})", args.name);
    }
    Ok(())
}

pub async fn run_category_rename(args: CategoryRenameArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    svc.rename_category(args.id, &args.name)?;
    if json {
        println!("{{\"renamed\":{}}}", args.id);
    } else {
        println!("✅ 已重命名分类 id={} 为「{}」", args.id, args.name);
    }
    Ok(())
}

pub async fn run_category_delete(args: CategoryDeleteArgs, json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    if !args.yes && !json {
        print!("确认删除分类 id={}？其下内容保留，仅解除绑定。[y/N] ", args.id);
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        if !buf.trim().eq_ignore_ascii_case("y") {
            println!("已取消");
            return Ok(());
        }
    }
    svc.delete_category(args.id)?;
    if json {
        println!("{{\"removed\":{}}}", args.id);
    } else {
        println!("🗑️  已删除分类 id={}", args.id);
    }
    Ok(())
}
