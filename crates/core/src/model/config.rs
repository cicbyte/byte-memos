//! 配置数据类型（config.toml）

use serde::{Deserialize, Serialize};

/// 用户配置（config.toml，固定位于 app_root/config/）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 数据目录路径（None = 默认 app_root/data/）
    #[serde(default)]
    pub data_dir: Option<String>,
    #[serde(default)]
    pub backup: BackupConfig,
    #[serde(default)]
    pub api: ApiConfig,
    /// 用户偏好（导入排除目录等）
    #[serde(default)]
    pub preferences: PreferencesConfig,
}

/// 备份相关配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// 默认导出路径（None = 用 data_root/backups/）
    #[serde(default)]
    pub output_dir: Option<String>,
    /// 导出时是否压缩（true=zip，false=目录快照）
    #[serde(default = "default_true")]
    pub compress: bool,
}

fn default_true() -> bool {
    true
}

/// API 服务相关配置（REST API + MCP 各有独立 Key）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// REST API Key（认证用，首次启动自动生成）
    #[serde(default)]
    pub key: String,
    /// MCP HTTP 服务 Key（独立于 REST API）
    #[serde(default)]
    pub mcp_key: String,
    /// 本地访问（127.0.0.1）是否跳过认证
    #[serde(default = "default_true")]
    pub allow_local_no_key: bool,
}

/// 用户偏好配置（导入行为等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesConfig {
    /// 导入项目时排除的目录名（按名精确匹配，递归跳过整个子树）。
    /// 默认预置常见依赖/构建/IDE 目录。
    #[serde(default = "default_exclude_dirs")]
    pub exclude_dirs: Vec<String>,
}

/// 预置的默认排除目录（常见依赖、构建产物、IDE 配置、系统垃圾文件）
fn default_exclude_dirs() -> Vec<String> {
    vec![
        "node_modules".into(),
        ".git".into(),
        "target".into(),
        "dist".into(),
        "build".into(),
        ".next".into(),
        ".nuxt".into(),
        "out".into(),
        ".output".into(),
        "__pycache__".into(),
        ".venv".into(),
        "venv".into(),
        ".idea".into(),
        ".vscode".into(),
        ".DS_Store".into(),
    ]
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: None,
            backup: BackupConfig::default(),
            api: ApiConfig::default(),
            preferences: PreferencesConfig::default(),
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            output_dir: None,
            compress: true,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            key: String::new(),
            mcp_key: String::new(),
            allow_local_no_key: true,
        }
    }
}

impl Default for PreferencesConfig {
    fn default() -> Self {
        Self {
            exclude_dirs: default_exclude_dirs(),
        }
    }
}
