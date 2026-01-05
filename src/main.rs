use anyhow::{Context, Result};
use clap::Parser;
use console::{style, Color};
use dialoguer::{theme::ColorfulTheme, Input};
use reqwest::Client;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use tokio::time::{timeout, Duration};

// API 配置
const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const MODEL: &str = "claude-sonnet-4-5-20250929";

#[derive(Parser, Debug)]
#[command(name = "rust-claude-code")]
#[command(about = "A Rust implementation of Claude Code CLI", long_about = None)]
#[command(version)]
struct Args {
    /// Anthropic API key (or set ANTHROPIC_API_KEY env variable)
    #[arg(short, long)]
    api_key: Option<String>,

    /// Maximum number of turns in conversation
    #[arg(short, long, default_value = "10")]
    max_turns: usize,

    /// Non-interactive mode: process a single prompt and exit
    #[arg(short, long)]
    prompt: Option<String>,
}

// Claude API 响应结构
#[derive(serde::Deserialize)]
struct ClaudeResponse {
    id: String,
    role: String,
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
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

// 工具定义
fn get_tools() -> serde_json::Value {
    json!([
        {
            "name": "read_file",
            "description": "Read a file from the filesystem. Returns the file contents as a string.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file to read"
                    }
                },
                "required": ["file_path"]
            }
        },
        {
            "name": "write_file",
            "description": "Write content to a file, overwriting if it exists. Returns confirmation message.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["file_path", "content"]
            }
        },
        {
            "name": "execute_command",
            "description": "Execute a shell command and return its output. Use for terminal operations like git, npm, cargo, etc.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    }
                },
                "required": ["command"]
            }
        },
        {
            "name": "list_files",
            "description": "List files in a directory using glob patterns",
            "input_schema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g., '*.rs', 'src/**/*.rs')"
                    },
                    "path": {
                        "type": "string",
                        "description": "Base directory path (defaults to current directory)"
                    }
                },
                "required": ["pattern"]
            }
        }
    ])
}

// 执行工具调用
async fn execute_tool(
    name: &str,
    input: &serde_json::Value,
) -> Result<String> {
    match name {
        "read_file" => {
            let file_path = input["file_path"]
                .as_str()
                .context("Missing file_path")?;

            let content = fs::read_to_string(file_path)
                .with_context(|| format!("Failed to read file: {}", file_path))?;

            Ok(content)
        }

        "write_file" => {
            let file_path = input["file_path"]
                .as_str()
                .context("Missing file_path")?;
            let content = input["content"]
                .as_str()
                .context("Missing content")?;

            // 确保父目录存在
            if let Some(parent) = Path::new(file_path).parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }

            fs::write(file_path, content)
                .with_context(|| format!("Failed to write file: {}", file_path))?;

            Ok(format!("Successfully wrote to file: {}", file_path))
        }

        "execute_command" => {
            let command = input["command"]
                .as_str()
                .context("Missing command")?;

            println!("\n{}", style("Executing:").cyan());
            println!("  {}", style(command).yellow());

            let output = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args(["/C", command])
                    .output()?
            } else {
                Command::new("sh")
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
                    result.push_str("\n");
                }
                result.push_str(&stderr);
            }

            if result.is_empty() {
                result = "(command produced no output)".to_string();
            }

            Ok(result)
        }

        "list_files" => {
            let pattern = input["pattern"]
                .as_str()
                .context("Missing pattern")?;
            let base_path = input["path"]
                .as_str()
                .unwrap_or(".");

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

// 调用 Claude API
async fn call_claude(
    client: &Client,
    api_key: &str,
    messages: &serde_json::Value,
    tools: bool,
) -> Result<ClaudeResponse> {
    let mut request_body = json!({
        "model": MODEL,
        "max_tokens": 8192,
        "messages": messages
    });

    if tools {
        request_body["tools"] = get_tools();
    }

    let response = client
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", API_VERSION)
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .context("Failed to call Claude API")?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("API error: {}", error_text);
    }

    let claude_response: ClaudeResponse = response.json().await?;
    Ok(claude_response)
}

// 处理工具使用并继续对话
async fn process_tool_use(
    client: &Client,
    api_key: &str,
    messages: &mut Vec<serde_json::Value>,
    tool_use_id: &str,
    tool_name: &str,
    tool_input: &serde_json::Value,
) -> Result<()> {
    // 执行工具
    let tool_result = execute_tool(tool_name, tool_input).await?;

    // 添加工具结果到对话历史
    messages.push(json!({
        "role": "user",
        "content": [
            {
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": tool_result
            }
        ]
    }));

    // 继续对话
    let response = call_claude(client, api_key, &json!(messages), true).await?;

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
                let name = block.name.as_ref().context("Missing tool name")?;
                let id = block.id.as_ref().context("Missing tool id")?;
                let input = block.input.as_ref().context("Missing tool input")?;

                println!("\n{} {}",
                    style("Tool:").cyan(),
                    style(name).yellow()
                );

                // 递归处理工具使用
                process_tool_use(client, api_key, messages, id, name, input).await?;
            }
            _ => {}
        }
    }

    // 添加助手响应到历史
    let assistant_content: Vec<serde_json::Value> = response.content.iter().map(|block| {
        json!({
            "type": block.content_type,
            "text": block.text,
            "name": block.name,
            "id": block.id,
            "input": block.input
        })
    }).collect();

    messages.push(json!({
        "role": "assistant",
        "content": assistant_content
    }));

    Ok(())
}

// 运行对话循环
async fn run_conversation(args: Args, api_key: String) -> Result<()> {
    let client = Client::new();
    let mut messages: Vec<serde_json::Value> = Vec::new();
    let mut turn_count = 0;

    let theme = ColorfulTheme::default();

    loop {
        // 获取用户输入
        let user_input = if let Some(prompt) = &args.prompt {
            prompt.clone()
        } else {
            Input::with_theme(&theme)
                .with_prompt("You")
                .allow_empty(false)
                .interact()
                .unwrap()
        };

        // 添加用户消息到历史
        messages.push(json!({
            "role": "user",
            "content": user_input
        }));

        // 调用 Claude API
        let response = timeout(
            Duration::from_secs(120),
            call_claude(&client, &api_key, &json!(messages), true)
        )
        .await
        .context("Request timed out")??;

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

                    println!("\n{} {}",
                        style("Tool:").cyan(),
                        style(name).yellow()
                    );

                    // 添加助手消息到历史
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

                    // 处理工具使用
                    process_tool_use(&client, &api_key, &mut messages, id, name, input).await?;
                }
                _ => {}
            }
        }

        // 如果没有工具使用，添加助手响应到历史
        if !response.content.iter().any(|b| b.content_type == "tool_use") {
            let assistant_content: Vec<serde_json::Value> = response.content.iter().map(|block| {
                json!({
                    "type": block.content_type,
                    "text": block.text
                })
            }).collect();

            messages.push(json!({
                "role": "assistant",
                "content": assistant_content
            }));
        }

        turn_count += 1;

        // 检查是否应该退出
        if args.prompt.is_some() {
            break;
        }

        if turn_count >= args.max_turns {
            println!("\n{}", style("Maximum turns reached.").dim());
            break;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 获取 API key
    let api_key = if let Some(key) = args.api_key {
        key
    } else {
        std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|| {
            eprintln!("{}", style("Error: ANTHROPIC_API_KEY not found").red());
            eprintln!("Set it with: export ANTHROPIC_API_KEY=your_key");
            eprintln!("Or pass with: --api-key your_key");
            std::process::exit(1);
        })
    };

    println!("\n{}", style("🦀 Rust Claude Code").blue().bold());
    println!("{}", style("A Rust implementation of Claude Code").dim());
    println!();

    // 运行对话
    run_conversation(args, api_key).await?;

    Ok(())
}
