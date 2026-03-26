//! 模板管理命令：list、create、delete、from-template、save-as-template。

use clap::{Args, Subcommand};

use super::open_service;

#[derive(Subcommand, Debug)]
pub enum TemplateCommand {
    /// 列出所有模板
    List,
    /// 创建模板
    Create(TemplateCreateArgs),
    /// 删除模板
    Delete(TemplateDeleteArgs),
    /// 从模板创建笔记
    Use(TemplateUseArgs),
    /// 将笔记保存为模板
    SaveAs(TemplateSaveAsArgs),
}

#[derive(Args, Debug)]
pub struct TemplateCreateArgs {
    /// 模板名称
    pub name: String,
    /// 模板内容（Markdown）
    #[arg(long)]
    pub content: Option<String>,
    /// 模板描述
    #[arg(long, default_value = "")]
    pub description: String,
}

#[derive(Args, Debug)]
pub struct TemplateDeleteArgs {
    /// 模板名称
    pub name: String,
}

#[derive(Args, Debug)]
pub struct TemplateUseArgs {
    /// 模板名称
    pub name: String,
    /// 笔记标题（可选，默认使用模板名称）
    #[arg(long)]
    pub title: Option<String>,
}

#[derive(Args, Debug)]
pub struct TemplateSaveAsArgs {
    /// 笔记 shortId
    pub short_id: String,
    /// 模板名称
    pub name: String,
    /// 模板描述
    #[arg(long, default_value = "")]
    pub description: String,
}

pub async fn run_template_list(json: bool) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let templates = svc.list_templates()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&templates)?);
    } else {
        if templates.is_empty() {
            println!("暂无模板");
            return Ok(());
        }
        println!("模板列表（共 {} 个）：", templates.len());
        for t in &templates {
            let desc = if t.description.is_empty() { "" } else { &format!(" - {}", t.description) };
            println!("  📝 {}{}", t.name, desc);
            let content_preview: String = t.content.chars().take(60).collect();
            let suffix = if t.content.len() > 60 { "..." } else { "" };
            println!("     {}{}", content_preview, suffix);
        }
    }
    Ok(())
}

pub async fn run_template_create(json: bool, args: TemplateCreateArgs) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let content = args.content.unwrap_or_default();
    let id = svc.create_template(&args.name, &content, &args.description)?;

    if json {
        println!("{}", serde_json::json!({"ok": true, "id": id, "name": args.name}));
    } else {
        println!("✅ 模板「{}」已创建", args.name);
    }
    Ok(())
}

pub async fn run_template_delete(json: bool, args: TemplateDeleteArgs) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    svc.delete_template(&args.name)?;

    if json {
        println!("{}", serde_json::json!({"ok": true, "name": args.name}));
    } else {
        println!("✅ 模板「{}」已删除", args.name);
    }
    Ok(())
}

pub async fn run_template_use(json: bool, args: TemplateUseArgs) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let m = svc.create_memo_from_template(&args.name, args.title.as_deref())?;

    if json {
        println!("{}", serde_json::to_string(&m)?);
    } else {
        println!("✅ 已从模板「{}」创建笔记：{}", args.name, m.short_id);
    }
    Ok(())
}

pub async fn run_template_save_as(json: bool, args: TemplateSaveAsArgs) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let id = svc.save_memo_as_template(&args.short_id, &args.name, &args.description)?;

    if json {
        println!("{}", serde_json::json!({"ok": true, "id": id, "name": args.name}));
    } else {
        println!("✅ 笔记 {} 已保存为模板「{}」", args.short_id, args.name);
    }
    Ok(())
}
