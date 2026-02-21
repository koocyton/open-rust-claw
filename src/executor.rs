use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::process::Command;
use tracing::{error, info};

use crate::config::ExecutorConfig;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TaskCommand {
    /// 要执行的 shell 命令
    pub command: String,
    /// 命令说明
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct CommandResult {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub struct Executor {
    config: ExecutorConfig,
}

impl Executor {
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }

    /// 执行单条命令
    pub async fn run_command(&self, cmd: &str) -> Result<CommandResult> {
        info!(cmd = %cmd, "执行命令");

        let working_dir = self
            .config
            .working_dir
            .as_deref()
            .unwrap_or(".");

        let output = tokio::time::timeout(
            Duration::from_secs(self.config.timeout_secs),
            Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(working_dir)
                .output(),
        )
        .await
        .with_context(|| format!("命令超时 ({} 秒): {cmd}", self.config.timeout_secs))?
        .with_context(|| format!("命令执行失败: {cmd}"))?;

        let result = CommandResult {
            command: cmd.to_string(),
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        };

        if result.success {
            info!(cmd = %cmd, "命令执行成功");
        } else {
            error!(cmd = %cmd, code = ?result.exit_code, stderr = %result.stderr, "命令执行失败");
        }

        Ok(result)
    }

    /// 批量执行命令列表
    pub async fn run_commands(&self, commands: &[TaskCommand]) -> Vec<CommandResult> {
        let mut results = Vec::new();
        for task in commands {
            info!(desc = %task.description, cmd = %task.command, "执行任务");
            match self.run_command(&task.command).await {
                Ok(result) => {
                    let success = result.success;
                    results.push(result);
                    if !success {
                        info!("命令失败，停止后续执行");
                        break;
                    }
                }
                Err(e) => {
                    error!(err = %e, "命令执行异常");
                    results.push(CommandResult {
                        command: task.command.clone(),
                        success: false,
                        exit_code: None,
                        stdout: String::new(),
                        stderr: e.to_string(),
                    });
                    break;
                }
            }
        }
        results
    }
}
