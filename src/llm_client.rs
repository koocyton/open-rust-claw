use anyhow::{Context, Result};
use serde_json::{json, Value};
use tracing::{debug, info};

use crate::config::LlmConfig;

const DEFAULT_SYSTEM_PROMPT: &str = r#"你是一个自动化任务执行代理。用户通过 Telegram 频道发来消息，你需要分析用户的意图，返回要执行的 shell 命令列表。

你拥有一台服务器的完全控制权，可以执行任何 shell 命令。

请返回一个 JSON 数组，每个元素包含：
- "command": 要执行的 shell 命令（字符串）
- "description": 这条命令做什么的简短说明（字符串）

只返回 JSON 数组，不要包含其他文字。如果消息不需要执行任何命令，返回空数组 []。

示例：
[
  {"command": "df -h", "description": "检查磁盘空间"},
  {"command": "free -m", "description": "检查内存使用"}
]"#;

pub struct LlmClient {
    client: reqwest::Client,
    config: LlmConfig,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    pub async fn chat(&self, user_message: &str) -> Result<String> {
        let system_prompt = self
            .config
            .system_prompt
            .as_deref()
            .unwrap_or(DEFAULT_SYSTEM_PROMPT);

        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let body = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_message },
            ]
        });

        info!(model = %self.config.model, "调用 LLM");
        debug!(url = %url, body = %body, "LLM 请求");

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("LLM API 请求失败")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM API 错误 {status}: {text}");
        }

        let result: Value = resp.json().await.context("LLM 响应解析失败")?;
        debug!(response = %result, "LLM 响应");

        let content = result
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(content)
    }
}
