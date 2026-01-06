use anyhow::{Context, Result};
use clap::Parser;
use console::style;
use dialoguer::{theme::ColorfulTheme, Input};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

mod config;
mod error;

use config::Config;
use error::ApiClient;

const MODEL: &str = "claude-3-haiku-20240307";

const MAX_CONVERSATION_HISTORY: usize = 50;

#[derive(Debug, Serialize, Deserialize)]
struct ConversationHistory {
    metadata: ConversationMetadata,
    messages: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConversationMetadata {
    created_at: u64,
    version: String,
    model: String,
}

fn create_conversation_history(messages: &[serde_json::Value]) -> ConversationHistory {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    ConversationHistory {
        metadata: ConversationMetadata {
            created_at: now,
            version: "0.1.0".to_string(),
            model: MODEL.to_string(),
        },
        messages: messages.to_vec(),
    }
}

async fn save_conversation_history(
    messages: &[serde_json::Value],
    config: &Config,
) -> Result<PathBuf> {
    if !config.user_settings.auto_save {
        return Ok(PathBuf::new());
    }

    let history = create_conversation_history(messages);
    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let filename = format!("conversation_{}.json", timestamp);
    let claude_dir = std::env::current_dir()?.join(".claude");
    let history_file = claude_dir.join("history").join(filename);

    fs::create_dir_all(history_file.parent().unwrap())
        .context("Failed to create history directory")?;

    let content = serde_json::to_string_pretty(&history)
        .context("Failed to serialize conversation history")?;

    fs::write(&history_file, content).context("Failed to write conversation history")?;

    info!("Conversation history saved to: {}", history_file.display());
    Ok(history_file)
}

#[derive(Parser, Debug)]
#[command(name = "rust-claude-code")]
#[command(about = "A Rust implementation of Claude Code CLI", long_about = None)]
#[command(version)]
struct Args {
    /// Anthropic API key (overrides config and environment)
    #[arg(short, long)]
    api_key: Option<String>,

    /// Maximum number of turns in conversation
    #[arg(short, long, default_value = "10")]
    max_turns: usize,

    /// Non-interactive mode: process a single prompt and exit
    #[arg(short, long)]
    prompt: Option<String>,

    /// API base URL (overrides config)
    #[arg(short = 'u', long)]
    api_url: Option<String>,

    /// Timeout in seconds (overrides config)
    #[arg(short = 't', long)]
    timeout: Option<u64>,

    /// Show configuration file path
    #[arg(long)]
    show_config: bool,
}

// Claude API 响应结构
#[derive(serde::Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(serde::Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
    name: Option<String>,
    id: Option<String>,
    input: Option<serde_json::Value>,
}

// 限制对话历史长度以防止内存泄漏
fn trim_conversation_history(messages: &mut Vec<serde_json::Value>) {
    if messages.len() > MAX_CONVERSATION_HISTORY {
        // 保留前几条重要的系统消息，删除中间的消息
        let system_messages_count = messages
            .iter()
            .take_while(|msg| msg["role"] == "system")
            .count();

        if system_messages_count < messages.len() {
            // 删除中间的消息，保留系统消息和最近的消息
            let keep_start = system_messages_count;
            let remove_end = messages
                .len()
                .saturating_sub(MAX_CONVERSATION_HISTORY - system_messages_count);

            if remove_end > keep_start {
                messages.drain(keep_start..remove_end);
            }
        }
    }
}

// 执行工具调用
async fn execute_tool(name: &str, input: &serde_json::Value) -> Result<String> {
    match name {
        "read_file" => {
            let file_path = input["file_path"].as_str().context("Missing file_path")?;
            let content = std::fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {}", file_path))?;
            Ok(content)
        }
        "write_file" => {
            let file_path = input["file_path"].as_str().context("Missing file_path")?;
            let content = input["content"].as_str().context("Missing content")?;

            if let Some(parent) = std::path::Path::new(file_path).parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }

            std::fs::write(file_path, content)
                .with_context(|| format!("Failed to write file: {}", file_path))?;

            Ok(format!("Successfully wrote to file: {}", file_path))
        }
        "execute_command" => {
            let command = input["command"].as_str().context("Missing command")?;

            println!("\n{}", style("Executing:").cyan());
            println!("  {}", style(command).yellow());

            let output = if cfg!(target_os = "windows") {
                std::process::Command::new("cmd")
                    .args(["/C", command])
                    .output()?
            } else {
                std::process::Command::new("sh")
                    .args(["-c", command])
                    .output()?
            };

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&stderr);
            }

            if result.is_empty() {
                result = "(command produced no output)".to_string();
            }

            Ok(result)
        }
        "list_files" => {
            let pattern = input["pattern"].as_str().context("Missing pattern")?;
            let base_path = input["path"].as_str().unwrap_or(".");

            use glob::glob;

            let full_pattern = if pattern.starts_with('/') {
                pattern.to_string()
            } else {
                format!("{}/{}", base_path, pattern)
            };

            let mut files = Vec::new();

            for entry in glob(&full_pattern)
                .with_context(|| format!("Failed to read glob pattern: {}", full_pattern))?
            {
                match entry {
                    Ok(path) => {
                        if let Some(path_str) = path.to_str() {
                            files.push(path_str.to_string());
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading entry: {:?}", e);
                    }
                }
            }

            if files.is_empty() {
                Ok("(no files found)".to_string())
            } else {
                Ok(files.join("\n"))
            }
        }
        _ => Ok(format!("Unknown tool: {}", name)),
    }
}

async fn call_claude(
    api_client: &ApiClient,
    messages: &serde_json::Value,
    tools: bool,
) -> Result<ClaudeResponse> {
    let response_json = api_client.call_claude_with_retry(messages, tools).await?;

    let claude_response: ClaudeResponse = serde_json::from_value(response_json)?;
    Ok(claude_response)
}

// 工具使用任务结构
struct ToolUseTask {
    tool_use_id: String,
    tool_name: String,
    tool_input: serde_json::Value,
}

async fn process_tool_use(
    api_client: &ApiClient,
    messages: &mut Vec<serde_json::Value>,
    initial_task: ToolUseTask,
) -> Result<()> {
    let mut task_stack = vec![initial_task];

    while let Some(task) = task_stack.pop() {
        let tool_result = execute_tool(&task.tool_name, &task.tool_input).await?;

        messages.push(json!({
            "role": "user",
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": task.tool_use_id,
                    "content": tool_result
                }
            ]
        }));

        trim_conversation_history(messages);

        let response = call_claude(api_client, &json!(messages), true).await?;

        // 收集新的工具使用任务
        let mut new_tool_tasks = Vec::new();

        // 处理响应中的所有内容块
        for block in &response.content {
            match block.content_type.as_str() {
                "text" => {
                    if let Some(text) = &block.text {
                        println!("\n{}", style("Claude:").green());
                        println!("{}", text);
                    }
                }
                "tool_use" => {
                    let name = block.name.as_ref().context("Missing tool name")?.clone();
                    let id = block.id.as_ref().context("Missing tool id")?.clone();
                    let input = block.input.as_ref().context("Missing tool input")?.clone();

                    println!("\n{} {}", style("Tool:").cyan(), style(&name).yellow());

                    // 将新任务添加到临时列表
                    new_tool_tasks.push(ToolUseTask {
                        tool_use_id: id,
                        tool_name: name,
                        tool_input: input,
                    });
                }
                _ => {}
            }
        }

        // 添加助手响应到历史
        let assistant_content: Vec<serde_json::Value> = response
            .content
            .iter()
            .map(|block| {
                json!({
                    "type": block.content_type,
                    "text": block.text,
                    "name": block.name,
                    "id": block.id,
                    "input": block.input
                })
            })
            .collect();

        messages.push(json!({
            "role": "assistant",
            "content": assistant_content
        }));

        // 限制对话历史长度
        trim_conversation_history(messages);

        // 将新工具任务添加到栈中（逆序添加以保持执行顺序）
        for task in new_tool_tasks.into_iter().rev() {
            task_stack.push(task);
        }
    }

    Ok(())
}

async fn run_conversation(args: Args, config: &Config) -> Result<()> {
    info!("Starting conversation");
    info!("API base URL: {}", config.api_base_url);
    info!(
        "Timeout: {} seconds",
        args.timeout.unwrap_or(config.api_timeout_ms / 1000)
    );

    let api_client = ApiClient::new(config.api_key.clone(), config.api_base_url.clone());
    let stats = api_client.get_stats();
    let mut messages: Vec<serde_json::Value> = Vec::new();
    let mut turn_count = 0;

    let timeout_secs = args.timeout.unwrap_or(config.api_timeout_ms / 1000);
    let max_turns = args.max_turns;

    let theme = ColorfulTheme::default();

    loop {
        let user_input = if let Some(prompt) = &args.prompt {
            info!("Using single prompt mode");
            prompt.clone()
        } else {
            Input::with_theme(&theme)
                .with_prompt("You")
                .allow_empty(false)
                .interact()
                .unwrap()
        };

        info!(
            "User input received (turn {}/{})",
            turn_count + 1,
            max_turns
        );

        messages.push(json!({
            "role": "user",
            "content": user_input
        }));

        let response = timeout(
            Duration::from_secs(timeout_secs),
            call_claude(&api_client, &json!(messages), true),
        )
        .await
        .context("Request timed out")?
        .context("API call failed")?;

        // 处理响应
        for block in &response.content {
            match block.content_type.as_str() {
                "text" => {
                    if let Some(text) = &block.text {
                        println!("\n{}", style("Claude:").green());
                        println!("{}", text);
                    }
                }
                "tool_use" => {
                    let name = block.name.as_ref().context("Missing tool name")?;
                    let id = block.id.as_ref().context("Missing tool id")?;
                    let input = block.input.as_ref().context("Missing tool input")?;

                    info!("Tool execution requested: {}", name);
                    println!("\n{} {}", style("Tool:").cyan(), style(name).yellow());

                    let assistant_content = json!([{
                        "type": "tool_use",
                        "id": id,
                        "name": name,
                        "input": input
                    }]);

                    messages.push(json!({
                        "role": "assistant",
                        "content": assistant_content
                    }));

                    process_tool_use(
                        &api_client,
                        &mut messages,
                        ToolUseTask {
                            tool_use_id: id.clone(),
                            tool_name: name.clone(),
                            tool_input: input.clone(),
                        },
                    )
                    .await?;
                }
                _ => {}
            }
        }

        // 如果没有工具使用，添加助手响应到历史
        if !response
            .content
            .iter()
            .any(|b| b.content_type == "tool_use")
        {
            let assistant_content: Vec<serde_json::Value> = response
                .content
                .iter()
                .map(|block| {
                    json!({
                        "type": block.content_type,
                        "text": block.text
                    })
                })
                .collect();

            messages.push(json!({
                "role": "assistant",
                "content": assistant_content
            }));
        }

        turn_count += 1;

        if args.prompt.is_some() {
            info!("Single prompt mode completed");
            break;
        }

        if turn_count >= args.max_turns {
            info!("Maximum turns ({}) reached", args.max_turns);
            println!("\n{}", style("Maximum turns reached.").dim());
            break;
        }
    }

    info!("Conversation completed ({} turns)", turn_count);

    save_conversation_history(&messages, config).await?;

    let total_requests = stats
        .total_requests
        .load(std::sync::atomic::Ordering::SeqCst);
    let successful_requests = stats
        .successful_requests
        .load(std::sync::atomic::Ordering::SeqCst);
    let failed_requests = stats
        .failed_requests
        .load(std::sync::atomic::Ordering::SeqCst);
    let avg_duration = stats.average_duration_ms();
    let success_rate = stats.success_rate();

    info!("Performance statistics:");
    info!("  Total requests: {}", total_requests);
    info!("  Successful: {}", successful_requests);
    info!("  Failed: {}", failed_requests);
    info!("  Success rate: {:.2}%", success_rate);
    info!("  Average response time: {:.2} ms", avg_duration);

    println!("\n{}", style("Performance Statistics:").cyan());
    println!("  Total requests: {}", total_requests);
    println!(
        "  Successful: {}",
        style(format!("{}", successful_requests)).green()
    );
    if failed_requests > 0 {
        println!("  Failed: {}", style(format!("{}", failed_requests)).red());
    } else {
        println!("  Failed: {}", failed_requests);
    }
    println!("  Success rate: {:.2}%", success_rate);
    println!("  Average response time: {:.2} ms", avg_duration);

    Ok(())
}

fn init_logging() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))?
        .add_directive("rust_claude_code=debug".parse()?);

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize logging: {}", e))?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.show_config {
        let claude_dir = std::env::current_dir()?.join(".claude");
        println!("{}", claude_dir.display());
        return Ok(());
    }

    init_logging()?;
    info!("Initializing Rust Claude Code");

    let config = Config::load()?;
    info!("Configuration loaded successfully");

    // 如果命令行提供了 API key，覆盖配置
    let api_key = if let Some(ref key) = args.api_key {
        key.clone()
    } else {
        config.api_key.clone()
    };

    // 更新配置中的 API key（如果命令行提供了）
    let final_config = Config { api_key, ..config };

    println!("\n{}", style("🦀 Rust Claude Code").blue().bold());
    println!("{}", style("A Rust implementation of Claude Code").dim());
    println!();

    if final_config.user_settings.ai_enabled {
        println!("AI 功能: {}", style("已启用").green());
    } else {
        println!("AI 功能: {}", style("已禁用").yellow());
    }
    println!("配置文件: {}", style(".claude/settings.json").dim());
    println!();

    run_conversation(args, &final_config).await?;

    info!("Application shutting down");
    Ok(())
}
