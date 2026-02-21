use anyhow::Result;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::{error, info, warn};

use crate::config::AppConfig;
use crate::executor::{CommandResult, Executor, TaskCommand};
use crate::llm_client::LlmClient;

fn parse_commands(llm_response: &str) -> Vec<TaskCommand> {
    let json_text = extract_json_array(llm_response);
    match serde_json::from_str::<Vec<TaskCommand>>(&json_text) {
        Ok(cmds) => cmds,
        Err(e) => {
            warn!(err = %e, text = %llm_response, "无法解析 LLM 返回的命令列表");
            Vec::new()
        }
    }
}

fn extract_json_array(text: &str) -> String {
    if let Some(start) = text.find("```") {
        let after_backticks = &text[start + 3..];
        let content_start = after_backticks.find('\n').map(|i| i + 1).unwrap_or(0);
        let content = &after_backticks[content_start..];
        if let Some(end) = content.find("```") {
            return content[..end].trim().to_string();
        }
    }
    if let Some(start) = text.find('[') {
        if let Some(end) = text.rfind(']') {
            return text[start..=end].to_string();
        }
    }
    text.trim().to_string()
}

fn format_results(commands: &[TaskCommand], results: &[CommandResult]) -> String {
    let mut msg = String::from("📋 任务执行报告\n\n");
    for (i, result) in results.iter().enumerate() {
        let desc = commands
            .get(i)
            .map(|c| c.description.as_str())
            .unwrap_or("未知");
        let status = if result.success { "✅" } else { "❌" };
        msg.push_str(&format!("{status} {desc}\n"));
        msg.push_str(&format!("  命令: {}\n", result.command));
        if !result.stdout.is_empty() {
            let stdout = truncate(&result.stdout, 500);
            msg.push_str(&format!("  输出:\n{stdout}\n"));
        }
        if !result.stderr.is_empty() {
            let stderr = truncate(&result.stderr, 300);
            msg.push_str(&format!("  错误:\n{stderr}\n"));
        }
        msg.push('\n');
    }
    msg
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...(截断)", &s[..max])
    }
}

async fn handle_message(
    bot: Bot,
    msg: Message,
    llm: Arc<LlmClient>,
    executor: Arc<Executor>,
    allowed_chats: Vec<i64>,
    echo_result: bool,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let from = msg
        .from
        .as_ref()
        .map(|u| u.first_name.as_str())
        .unwrap_or("unknown");
    let chat_type = format!("{:?}", msg.chat.kind);

    println!("========================================");
    println!(
        "[收到消息] chat_id: {}, 发送者: {}, 类型: {}",
        chat_id.0, from, chat_type
    );
    println!(
        "[消息内容] {:?}",
        msg.text().unwrap_or("<非文本消息>")
    );
    println!("========================================");

    info!(chat_id = chat_id.0, from = %from, "收到 Telegram 更新");

    if !allowed_chats.is_empty() && !allowed_chats.contains(&chat_id.0) {
        println!("[权限] chat_id {} 不在允许列表中，已忽略", chat_id.0);
        info!(chat_id = chat_id.0, "忽略未授权的聊天");
        return Ok(());
    }

    let text = match msg.text() {
        Some(t) => t.to_string(),
        None => {
            println!("[忽略] 非文本消息");
            info!("忽略非文本消息");
            return Ok(());
        }
    };

    println!("[处理] 开始处理消息: {}", text);
    info!(chat_id = chat_id.0, text = %text, "收到消息");

    bot.send_message(chat_id, "🔄 正在分析任务...")
        .await
        .ok();

    let commands = match llm.chat(&text).await {
        Ok(resp) => parse_commands(&resp),
        Err(e) => {
            error!(err = %e, "LLM 调用失败");
            bot.send_message(chat_id, format!("❌ LLM 调用失败: {e}"))
                .await
                .ok();
            return Ok(());
        }
    };

    if commands.is_empty() {
        bot.send_message(chat_id, "ℹ️ 该消息不需要执行任何命令")
            .await
            .ok();
        return Ok(());
    }

    let plan: String = commands
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}. {} → `{}`", i + 1, c.description, c.command))
        .collect::<Vec<_>>()
        .join("\n");
    bot.send_message(chat_id, format!("📝 执行计划:\n{plan}"))
        .await
        .ok();

    let results = executor.run_commands(&commands).await;

    if echo_result {
        let report = format_results(&commands, &results);
        bot.send_message(chat_id, report).await.ok();
    }

    Ok(())
}

pub async fn run(config: AppConfig) -> Result<()> {
    let bot = Bot::new(&config.telegram.bot_token);
    let allowed_chats = config.telegram.allowed_chat_ids.clone();
    let echo_result = config.executor.echo_result;

    let llm = Arc::new(LlmClient::new(config.llm.clone()));
    let executor = Arc::new(Executor::new(config.executor.clone()));

    info!("开始监听 Telegram 消息...");
    info!("Bot Token: {}...", &config.telegram.bot_token[..config.telegram.bot_token.len().min(10)]);
    info!("允许的聊天 ID: {:?}", &config.telegram.allowed_chat_ids);

    let handler = dptree::entry()
        .branch(
            Update::filter_message().endpoint(
                |bot: Bot,
                 msg: Message,
                 llm: Arc<LlmClient>,
                 executor: Arc<Executor>,
                 allowed_chats: Vec<i64>,
                 echo_result: bool| {
                    handle_message(bot, msg, llm, executor, allowed_chats, echo_result)
                },
            ),
        )
        .branch(
            Update::filter_channel_post().endpoint(
                |bot: Bot,
                 msg: Message,
                 llm: Arc<LlmClient>,
                 executor: Arc<Executor>,
                 allowed_chats: Vec<i64>,
                 echo_result: bool| {
                    handle_message(bot, msg, llm, executor, allowed_chats, echo_result)
                },
            ),
        );

    println!("[启动] 先用 deleteWebhook 清理状态...");
    let delete_url = format!(
        "https://api.telegram.org/bot{}/deleteWebhook?drop_pending_updates=true",
        &config.telegram.bot_token
    );
    match reqwest::get(&delete_url).await {
        Ok(resp) => println!("[启动] deleteWebhook 响应: {}", resp.status()),
        Err(e) => println!("[启动] deleteWebhook 失败: {}", e),
    }

    println!("[启动] 开始 polling 循环...");

    let llm_clone = llm.clone();
    let executor_clone = executor.clone();

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![llm_clone, executor_clone, allowed_chats, echo_result])
        .default_handler(|upd| async move {
            println!("[默认处理] 收到未匹配的更新类型: {:?}", upd.kind);
            warn!("未处理的更新: {:?}", upd.kind);
        })
        .error_handler(LoggingErrorHandler::with_custom_text("消息处理出错"))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
