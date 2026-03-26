//! 标签管理命令：list（树形）、set-color、delete。

use anyhow::Result;
use clap::Subcommand;

use super::open_service;

#[derive(Subcommand, Debug)]
pub enum TagCommand {
    /// 列出所有标签（树形结构）
    List,
    /// 设置标签颜色
    SetColor {
        /// 标签名
        name: String,
        /// 颜色（如 #FF0000、red，留空则清除颜色）
        color: Option<String>,
    },
    /// 删除标签（关联笔记的标签会被级联删除）
    Delete {
        /// 标签名
        name: String,
    },
}

pub async fn run_tag_list(json: bool) -> Result<()> {
    let svc = open_service(json)?;
    let tags = svc.list_tags_tree()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&tags)?);
    } else {
        if tags.is_empty() {
            println!("暂无标签");
            return Ok(());
        }
        // 按 parent_id 分组显示树形结构
        let roots: Vec<_> = tags.iter().filter(|t| t.parent_id.is_none()).collect();
        for root in &roots {
            let color_mark = root.color.as_deref().unwrap_or("");
            let color_display = if color_mark.is_empty() {
                String::new()
            } else {
                format!(" [{color_mark}]")
            };
            println!(
                "  {}{} ({}篇笔记)",
                root.name, color_display, root.count
            );
            // 显示子标签
            let children: Vec<_> = tags.iter().filter(|t| t.parent_id == Some(root.id)).collect();
            for child in &children {
                let child_color = child.color.as_deref().unwrap_or("");
                let child_color_display = if child_color.is_empty() {
                    String::new()
                } else {
                    format!(" [{child_color}]")
                };
                println!(
                    "    {}/{}{} ({}篇笔记)",
                    root.name, child.name, child_color_display, child.count
                );
            }
        }
    }
    Ok(())
}

pub async fn run_tag_set_color(json: bool, name: String, color: Option<String>) -> Result<()> {
    let svc = open_service(json)?;
    svc.set_tag_color(&name, color.as_deref())?;

    if json {
        println!("{}", serde_json::json!({"ok": true, "tag": name, "color": color}));
    } else {
        match &color {
            Some(c) => println!("标签「{name}」颜色已设置为 {c}"),
            None => println!("标签「{name}」颜色已清除"),
        }
    }
    Ok(())
}

pub async fn run_tag_delete(json: bool, name: String) -> Result<()> {
    let svc = open_service(json)?;
    svc.delete_tag(&name)?;

    if json {
        println!("{}", serde_json::json!({"ok": true, "tag": name}));
    } else {
        println!("标签「{name}」已删除");
    }
    Ok(())
}
