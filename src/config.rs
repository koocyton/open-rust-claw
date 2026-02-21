use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub telegram: TelegramConfig,
    pub llm: LlmConfig,
    #[serde(default)]
    pub executor: ExecutorConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    /// 允许接收消息的聊天 ID 列表（频道/群组/用户），留空则接收所有
    #[serde(default)]
    pub allowed_chat_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    /// OpenAI 兼容 API 的 base URL
    pub base_url: String,
    /// API Key
    pub api_key: String,
    /// 模型名称
    pub model: String,
    /// 系统提示词（可选，有默认值）
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// 最大 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_max_tokens() -> u32 {
    2048
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExecutorConfig {
    /// 命令执行的工作目录
    pub working_dir: Option<String>,
    /// 单条命令最大执行时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// 是否在 Telegram 中回显执行结果
    #[serde(default = "default_true")]
    pub echo_result: bool,
}

fn default_timeout() -> u64 {
    120
}

fn default_true() -> bool {
    true
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            working_dir: None,
            timeout_secs: default_timeout(),
            echo_result: true,
        }
    }
}

impl AppConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("无法读取配置文件: {}", path.as_ref().display()))?;
        let config: AppConfig =
            toml::from_str(&content).with_context(|| "配置文件解析失败")?;
        Ok(config)
    }
}
