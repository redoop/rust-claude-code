use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// 用户配置文件结构 (.claude/settings.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    /// 界面主题
    #[serde(default = "default_theme")]
    pub theme: String,

    /// 自动保存功能
    #[serde(default = "default_auto_save")]
    pub auto_save: bool,

    /// AI 功能开关
    #[serde(default = "default_ai_enabled")]
    pub ai_enabled: bool,

    /// Anthropic API 密钥
    #[serde(default)]
    pub anthropic_api_key: Option<String>,

    /// API 基础 URL
    #[serde(default)]
    pub api_base_url: Option<String>,

    /// 置信度阈值
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f64,

    /// 启用的插件列表
    #[serde(default)]
    pub enabled_plugins: Vec<String>,
}

/// 本地配置文件结构 (.claude/settings.local.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalSettings {
    /// API 认证令牌
    pub anthropic_auth_token: Option<String>,

    /// 用户特定的覆盖配置
    #[serde(flatten)]
    pub overrides: serde_json::Value,
}

/// 应用配置，合并了所有配置源
#[derive(Debug, Clone)]
pub struct Config {
    pub user_settings: UserSettings,
    pub api_key: String,
    pub api_base_url: String,
    pub api_timeout_ms: u64,
}

// 默认值函数
fn default_theme() -> String {
    "default".to_string()
}

fn default_auto_save() -> bool {
    false
}

fn default_ai_enabled() -> bool {
    true
}

fn default_confidence_threshold() -> f64 {
    0.8
}

impl Default for UserSettings {
    fn default() -> Self {
        UserSettings {
            theme: default_theme(),
            auto_save: default_auto_save(),
            ai_enabled: default_ai_enabled(),
            anthropic_api_key: None,
            api_base_url: None,
            confidence_threshold: default_confidence_threshold(),
            enabled_plugins: vec!["rust-analyzer-lsp@claude-plugins-official".to_string()],
        }
    }
}

impl Default for LocalSettings {
    fn default() -> Self {
        LocalSettings {
            anthropic_auth_token: None,
            overrides: serde_json::json!({}),
        }
    }
}

impl Config {
    /// 加载配置，按优先级合并各个配置源
    pub fn load() -> Result<Self> {
        // 1. 加载用户配置
        let user_settings = Self::load_user_settings().unwrap_or_else(|_| UserSettings::default());

        // 2. 加载本地配置
        let local_settings =
            Self::load_local_settings().unwrap_or_else(|_| LocalSettings::default());

        // 3. 从环境变量加载配置
        let api_key = Self::get_api_key(&user_settings, &local_settings)?;
        let api_base_url = Self::get_api_base_url(&user_settings);
        let api_timeout_ms = Self::get_api_timeout();

        Ok(Config {
            user_settings,
            api_key,
            api_base_url,
            api_timeout_ms,
        })
    }

    /// 获取 .claude 目录路径
    fn get_claude_dir() -> Result<PathBuf> {
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;

        Ok(current_dir.join(".claude"))
    }

    /// 加载用户配置文件
    fn load_user_settings() -> Result<UserSettings> {
        let claude_dir = Self::get_claude_dir()?;
        let settings_path = claude_dir.join("settings.json");

        if !settings_path.exists() {
            // 创建默认配置文件
            fs::create_dir_all(&claude_dir)
                .with_context(|| format!("Failed to create directory: {:?}", claude_dir))?;

            let default_settings = UserSettings::default();
            let settings_content = serde_json::to_string_pretty(&default_settings)
                .context("Failed to serialize default settings")?;

            fs::write(&settings_path, settings_content)
                .with_context(|| format!("Failed to write settings file: {:?}", settings_path))?;

            return Ok(default_settings);
        }

        let content = fs::read_to_string(&settings_path)
            .with_context(|| format!("Failed to read settings file: {:?}", settings_path))?;

        let settings: UserSettings = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse settings file: {:?}", settings_path))?;

        Ok(settings)
    }

    /// 加载本地配置文件
    fn load_local_settings() -> Result<LocalSettings> {
        let claude_dir = Self::get_claude_dir()?;
        let settings_path = claude_dir.join("settings.local.json");

        if !settings_path.exists() {
            return Ok(LocalSettings::default());
        }

        let content = fs::read_to_string(&settings_path)
            .with_context(|| format!("Failed to read local settings file: {:?}", settings_path))?;

        let settings: LocalSettings = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse local settings file: {:?}", settings_path))?;

        Ok(settings)
    }

    /// 获取 API 密钥（按优先级）
    fn get_api_key(user_settings: &UserSettings, local_settings: &LocalSettings) -> Result<String> {
        // 优先级：命令行参数 > 环境变量 > 本地配置 > 用户配置
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            return Ok(key);
        }

        if let Ok(key) = std::env::var("ANTHROPIC_AUTH_TOKEN") {
            return Ok(key);
        }

        if let Some(key) = &local_settings.anthropic_auth_token {
            if !key.is_empty() {
                return Ok(key.clone());
            }
        }

        if let Some(key) = &user_settings.anthropic_api_key {
            if !key.is_empty() {
                return Ok(key.clone());
            }
        }

        anyhow::bail!(
            "API key not found. Please set ANTHROPIC_API_KEY environment variable \
            or configure it in .claude/settings.json"
        )
    }

    /// 获取 API 基础 URL
    fn get_api_base_url(user_settings: &UserSettings) -> String {
        if let Some(url) = &user_settings.api_base_url {
            if !url.is_empty() {
                return url.clone();
            }
        }

        std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com".to_string())
    }

    /// 获取 API 超时时间
    fn get_api_timeout() -> u64 {
        std::env::var("API_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120_000) // 默认 120 秒
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = UserSettings::default();
        assert_eq!(settings.theme, "default");
        assert!(!settings.auto_save);
        assert!(settings.ai_enabled);
        assert_eq!(settings.confidence_threshold, 0.8);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = UserSettings::default();
        let json = serde_json::to_string(&settings).unwrap();
        let _deserialized: UserSettings = serde_json::from_str(&json).unwrap();
    }
}
