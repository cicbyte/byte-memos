//! 附件管理命令：upload、list、delete、download。

use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::open_service;

#[derive(Subcommand, Debug)]
pub enum AttachmentCommand {
    /// 上传附件到笔记
    Upload(AttachmentUploadArgs),
    /// 列出笔记的所有附件
    List(AttachmentListArgs),
    /// 删除附件
    Delete(AttachmentDeleteArgs),
}

#[derive(Args, Debug)]
pub struct AttachmentUploadArgs {
    /// 笔记 shortId
    pub memo_short_id: String,
    /// 要上传的文件路径
    pub file: PathBuf,
}

#[derive(Args, Debug)]
pub struct AttachmentListArgs {
    /// 笔记 shortId
    pub memo_short_id: String,
}

#[derive(Args, Debug)]
pub struct AttachmentDeleteArgs {
    /// 附件 ID
    pub id: i64,
}

pub async fn run_attachment_upload(json: bool, args: AttachmentUploadArgs) -> anyhow::Result<()> {
    if !args.file.exists() {
        anyhow::bail!("文件不存在：{}", args.file.display());
    }
    let filename = args.file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    let svc = open_service(json)?;
    let id = svc.upload_attachment_file(&args.memo_short_id, &filename, &args.file)?;

    if json {
        println!("{}", serde_json::json!({"ok": true, "id": id, "filename": filename}));
    } else {
        println!("✅ 已上传附件「{}」(ID: {})", filename, id);
    }
    Ok(())
}

pub async fn run_attachment_list(json: bool, args: AttachmentListArgs) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    let attachments = svc.list_attachments(&args.memo_short_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&attachments)?);
    } else {
        if attachments.is_empty() {
            println!("暂无附件");
            return Ok(());
        }
        println!("附件列表（共 {} 个）：", attachments.len());
        for att in &attachments {
            let size_str = format_size(att.file_size);
            println!("  📎 {} [{}] ({}): {}", att.filename, att.mime_type, size_str, att.id);
        }
    }
    Ok(())
}

pub async fn run_attachment_delete(json: bool, args: AttachmentDeleteArgs) -> anyhow::Result<()> {
    let svc = open_service(json)?;
    svc.delete_attachment(args.id)?;

    if json {
        println!("{}", serde_json::json!({"ok": true, "id": args.id}));
    } else {
        println!("✅ 附件 {} 已删除", args.id);
    }
    Ok(())
}

fn format_size(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
