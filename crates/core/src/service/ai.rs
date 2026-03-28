//! AI 业务：供应商管理（feature = "ai"）
//!
//! 安全模型（沿袭 byte-writer，经验证）：
//! - api_key 入库前 DPAPI 加密（dpapi: 前缀），前端永不回显明文，只拿 api_key_set 布尔
//! - 更新时 api_key 传空串 = 保留原值
//! - 启用为单选语义：首条自动启用；启用某条时事务内自动停用其余

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::ai::dpapi;
use crate::ai::factory::ProviderConfig;

use super::ProjectService;

/// 供应商列表项（不含 api_key 明文）
#[derive(Debug, Clone, Serialize)]
pub struct ProviderItem {
    pub id: i64,
    pub name: String,
    pub provider_key: String,
    pub base_url: String,
    /// 仅返回是否已设置，不回显明文
    pub api_key_set: bool,
    pub model: String,
    pub max_tokens: i64,
    pub temperature: f64,
    pub is_enabled: bool,
    pub sort_order: i64,
}

/// 添加供应商入参
#[derive(Debug, Clone, Deserialize)]
pub struct AddProviderInput {
    pub name: String,
    pub provider_key: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i64,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
}

fn default_max_tokens() -> i64 {
    4096
}
fn default_temperature() -> f64 {
    0.7
}

/// 更新供应商入参（api_key 为空表示保留原值）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProviderInput {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i64,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    pub is_enabled: bool,
}

/// 连通测试结果
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub ok: bool,
    pub message: String,
}

/// 内置常用供应商预设（Cherry Studio 模式：常驻列表，按需填 Key）
///
/// 端点核实来源：小米 MiMo / Token Plan 来自 byte-voice src-tauri（asr/mod.rs）；
/// GLM / GLM Coding Plan 为智谱开放平台公开端点；其余为官方 OpenAI 兼容端点。
pub struct BuiltinProvider {
    pub name: &'static str,
    pub provider_key: &'static str,
    pub base_url: &'static str,
    pub model: &'static str,
}

pub const BUILTIN_PROVIDERS: &[BuiltinProvider] = &[
    BuiltinProvider { name: "GLM 智谱", provider_key: "openai-compatible", base_url: "https://open.bigmodel.cn/api/paas/v4", model: "glm-4.6" },
    BuiltinProvider { name: "GLM Coding Plan", provider_key: "openai-compatible", base_url: "https://open.bigmodel.cn/api/coding/paas/v4", model: "glm-4.6" },
    BuiltinProvider { name: "DeepSeek", provider_key: "openai-compatible", base_url: "https://api.deepseek.com", model: "deepseek-chat" },
    BuiltinProvider { name: "Moonshot Kimi", provider_key: "openai-compatible", base_url: "https://api.moonshot.cn/v1", model: "kimi-k2-turbo-preview" },
    BuiltinProvider { name: "阿里云百炼 Qwen", provider_key: "openai-compatible", base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1", model: "qwen-plus" },
    BuiltinProvider { name: "硅基流动 SiliconFlow", provider_key: "openai-compatible", base_url: "https://api.siliconflow.cn/v1", model: "deepseek-ai/DeepSeek-V3" },
    BuiltinProvider { name: "小米 MiMo", provider_key: "openai-compatible", base_url: "https://api.xiaomimimo.com/v1", model: "mimo-v2.5" },
    BuiltinProvider { name: "小米 MiMo · Token Plan", provider_key: "openai-compatible", base_url: "https://token-plan-cn.xiaomimimo.com/v1", model: "mimo-v2.5" },
    BuiltinProvider { name: "OpenAI", provider_key: "openai-compatible", base_url: "https://api.openai.com/v1", model: "gpt-4o-mini" },
    BuiltinProvider { name: "Anthropic Claude", provider_key: "anthropic", base_url: "", model: "claude-sonnet-4-5" },
    BuiltinProvider { name: "Google Gemini", provider_key: "gemini", base_url: "", model: "gemini-2.5-flash" },
    BuiltinProvider { name: "Ollama（本地）", provider_key: "ollama", base_url: "http://localhost:11434", model: "qwen3:8b" },
];

fn row_to_item(r: crate::db::ai::AiProviderRow) -> ProviderItem {
    ProviderItem {
        id: r.id,
        name: r.name,
        provider_key: r.provider_key,
        base_url: r.base_url,
        api_key_set: !r.api_key.is_empty(),
        model: r.model,
        max_tokens: r.max_tokens,
        temperature: r.temperature,
        is_enabled: r.is_enabled,
        sort_order: r.sort_order,
    }
}

fn row_to_config(r: crate::db::ai::AiProviderRow) -> ProviderConfig {
    ProviderConfig {
        base_url: r.base_url,
        api_key: dpapi::unprotect(&r.api_key),
        model: r.model,
        max_tokens: r.max_tokens,
        temperature: r.temperature,
        provider_key: r.provider_key,
    }
}

impl ProjectService {
    /// 启动时种子：确保内置供应商常驻列表（按名称去重，幂等）。
    /// 用户改名/删除后下次启动会补回同名原始行（可再删，代价可忽略）。
    pub fn ai_provider_ensure_builtins(&self) -> Result<()> {
        for (i, b) in BUILTIN_PROVIDERS.iter().enumerate() {
            self.db.ai_provider_seed_if_missing(b.name, b.provider_key, b.base_url, b.model, i as i64)?;
        }
        Ok(())
    }

    /// 列出全部供应商（按 sort_order, id 排序）
    pub fn ai_provider_list(&self) -> Result<Vec<ProviderItem>> {
        Ok(self.db.ai_provider_list()?.into_iter().map(row_to_item).collect())
    }

    /// 添加供应商（首条自动启用）
    pub fn ai_provider_add(&self, input: &AddProviderInput) -> Result<i64> {
        if input.name.trim().is_empty() {
            return Err(anyhow!("名称不能为空"));
        }
        if crate::ai::factory::PROVIDER_KEYS.contains(&input.provider_key.as_str()) {
            // 合法
        } else {
            return Err(anyhow!(
                "不支持的供应商标识「{}」", input.provider_key
            ));
        }
        // 当前无任何启用供应商时，新记录自动启用（保证始终有可用供应商）
        let enabled_count = self
            .db
            .ai_provider_list()?
            .iter()
            .filter(|p| p.is_enabled)
            .count();
        let id = self.db.ai_provider_add(
            input.name.trim(),
            &input.provider_key,
            input.base_url.trim(),
            &dpapi::protect(&input.api_key).context("加密 api_key 失败")?,
            input.model.trim(),
            input.max_tokens,
            input.temperature,
            enabled_count == 0,
        )?;
        Ok(id)
    }

    /// 更新供应商（api_key 空串 = 保留原值；启用单选由 db 层事务保证）
    pub fn ai_provider_update(&self, input: &UpdateProviderInput) -> Result<()> {
        if input.name.trim().is_empty() {
            return Err(anyhow!("名称不能为空"));
        }
        let api_key_encrypted = if input.api_key.is_empty() {
            // 保留原值：取出原密文原样回传
            self.db
                .ai_provider_api_key(input.id)?
                .ok_or_else(|| anyhow!("AI 供应商不存在"))?
        } else {
            dpapi::protect(&input.api_key).context("加密 api_key 失败")?
        };
        self.db.ai_provider_update(
            input.id,
            input.name.trim(),
            input.base_url.trim(),
            &api_key_encrypted,
            input.model.trim(),
            input.max_tokens,
            input.temperature,
            input.is_enabled,
        )?;
        Ok(())
    }

    pub fn ai_provider_delete(&self, id: i64) -> Result<()> {
        self.db.ai_provider_delete(id)?;
        Ok(())
    }

    /// 唯一启用中的供应商配置（api_key 已解密）；未配置返回 None
    pub fn enabled_ai_provider_config(&self) -> Result<Option<ProviderConfig>> {
        Ok(self.db.ai_provider_enabled()?.map(row_to_config))
    }

    /// 取指定 id 供应商配置（api_key 已解密）
    pub fn ai_provider_config_by_id(&self, id: i64) -> Result<ProviderConfig> {
        self.db
            .ai_provider_by_id(id)?
            .map(row_to_config)
            .ok_or_else(|| anyhow!("AI 供应商不存在"))
    }
}

/// 从 LLM 输出中提取首个 JSON 数组片段（容忍前后多余文字）
pub fn extract_json_array(raw: &str) -> String {
    let s = raw.trim();
    if let Some(start) = s.find('[') {
        if let Some(end) = s.rfind(']') {
            if end > start {
                return s[start..=end].to_string();
            }
        }
    }
    s.to_string()
}

/// 从 LLM 输出中提取首个 JSON 对象片段（{ ... }）
pub fn extract_json_object(raw: &str) -> String {
    let s = raw.trim();
    if let Some(start) = s.find('{') {
        if let Some(end) = s.rfind('}') {
            if end > start {
                return s[start..=end].to_string();
            }
        }
    }
    s.to_string()
}

/// 解析标签建议（JSON 数组，降级按行分割；上限 8 个）
pub fn parse_tags(raw: &str) -> Result<Vec<String>> {
    let extracted = extract_json_array(raw);
    if let Ok(mut v) = serde_json::from_str::<Vec<String>>(&extracted) {
        if v.is_empty() {
            return Err(anyhow!("未能解析标签结果"));
        }
        if v.len() > 8 {
            v.truncate(8);
        }
        return Ok(v);
    }
    // 降级：按行分割
    let mut tags = Vec::new();
    for line in raw.lines() {
        let s = line
            .trim()
            .trim_matches(|c| matches!(c, '-' | '•' | '·' | '"' | '\'' | ',' | '，'));
        if !s.is_empty() && !s.starts_with('[') && !s.starts_with(']') {
            tags.push(s.to_string());
        }
    }
    if tags.is_empty() {
        Err(anyhow!("未能解析标签结果"))
    } else {
        tags.truncate(8);
        Ok(tags)
    }
}

/// 解析分类建议（JSON 对象 {category}，空串=无匹配）
pub fn parse_category(raw: &str) -> Result<String> {
    let extracted = extract_json_object(raw);
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&extracted) {
        if let Some(c) = v.get("category").and_then(|c| c.as_str()) {
            return Ok(c.trim().to_string());
        }
    }
    // 降级：非空单行文本直接视为分类名
    let line = raw.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    Ok(line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tags_handles_json_and_fallback() {
        let t = parse_tags(r#"["Go","Rust","后端"]"#).unwrap();
        assert_eq!(t, vec!["Go".to_string(), "Rust".into(), "后端".into()]);
        let t = parse_tags("好的：[\"a\",\"b\"]").unwrap();
        assert_eq!(t, vec!["a".to_string(), "b".into()]);
    }

    #[test]
    fn parse_tags_truncates_to_8() {
        let big: Vec<String> = (0..15).map(|i| format!("tag{i}")).collect();
        let raw = serde_json::to_string(&big).unwrap();
        assert_eq!(parse_tags(&raw).unwrap().len(), 8);
    }

    #[test]
    fn parse_category_variants() {
        assert_eq!(
            parse_category("{\"category\": \"工作\"}").unwrap(),
            "工作"
        );
        assert_eq!(parse_category("前置 {\"category\":\"学习\"} 后置").unwrap(), "学习");
        assert_eq!(parse_category("").unwrap(), "");
    }
}
