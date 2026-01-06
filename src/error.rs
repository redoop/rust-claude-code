use anyhow::{Context, Result};
use backoff::{future::retry, ExponentialBackoff};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

/// API 错误类型
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("API request failed with status {0}: {1}")]
    HttpError(u16, String),

    #[error("Rate limit exceeded, retry after {0} seconds")]
    RateLimit(u32),

    #[error("Authentication failed: invalid API key")]
    Authentication,

    #[error("Model overloaded: {0}")]
    Overloaded(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("Response parsing error: {0}")]
    ParseError(#[from] serde_json::Error),
}

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }
}

/// 带有重试机制的 API 客户端
pub struct ApiClient {
    client: Client,
    api_key: String,
    api_url: String,
    retry_config: RetryConfig,
    request_id: String,
}

impl ApiClient {
    pub fn new(api_key: String, api_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            api_url,
            retry_config: RetryConfig::default(),
            request_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// 调用 Claude API 并带有重试机制
    pub async fn call_claude_with_retry(
        &self,
        messages: &serde_json::Value,
        tools: bool,
    ) -> Result<serde_json::Value> {
        let request_id = self.request_id.clone();
        info!("Starting API call (request_id: {})", request_id);

        let backoff = ExponentialBackoff {
            initial_interval: self.retry_config.initial_delay,
            max_interval: self.retry_config.max_delay,
            multiplier: self.retry_config.multiplier,
            max_elapsed_time: Some(Duration::from_secs(120)), // 总超时时间
            ..Default::default()
        };

        let operation = || async {
            self.call_claude_once(messages, tools)
                .await
                .map_err(|e| match &e {
                    ApiError::RateLimit(_) => {
                        warn!("Rate limit hit, will retry (request_id: {})", request_id);
                        backoff::Error::transient(anyhow::anyhow!("{}", e))
                    }
                    ApiError::Overloaded(_) => {
                        warn!("Model overloaded, will retry (request_id: {})", request_id);
                        backoff::Error::transient(anyhow::anyhow!("{}", e))
                    }
                    ApiError::Network(_) => {
                        warn!("Network error, will retry (request_id: {})", request_id);
                        backoff::Error::transient(anyhow::anyhow!("{}", e))
                    }
                    ApiError::Timeout(_) => {
                        warn!("Timeout, will retry (request_id: {})", request_id);
                        backoff::Error::transient(anyhow::anyhow!("{}", e))
                    }
                    _ => {
                        error!("Non-retryable error (request_id: {}): {}", request_id, e);
                        backoff::Error::permanent(e.into())
                    }
                })
        };

        let result = retry(backoff, operation)
            .await
            .context("API call failed after all retries")?;

        info!("API call successful (request_id: {})", request_id);
        Ok(result)
    }

    /// 单次 API 调用
    async fn call_claude_once(
        &self,
        messages: &serde_json::Value,
        tools: bool,
    ) -> Result<serde_json::Value, ApiError> {
        let mut request_body = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 8192,
            "messages": messages
        });

        if tools {
            request_body["tools"] = get_tools();
        }

        let start_time = std::time::Instant::now();

        let response = self
            .client
            .post(&self.api_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("x-request-id", &self.request_id)
            .json(&request_body)
            .send()
            .await?;

        let elapsed = start_time.elapsed();
        info!("API request completed in {:?}", elapsed);

        let status = response.status();

        if status == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(60);
            return Err(ApiError::RateLimit(retry_after));
        }

        if status == 401 {
            return Err(ApiError::Authentication);
        }

        if status == 400 {
            let error_text = response.text().await?;
            return Err(ApiError::InvalidRequest(error_text));
        }

        if status == 529 {
            let error_text = response.text().await?;
            return Err(ApiError::Overloaded(error_text));
        }

        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(ApiError::HttpError(status.as_u16(), error_text));
        }

        let response_json: serde_json::Value = response.json().await?;
        Ok(response_json)
    }
}

/// 获取工具定义
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(1000));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.multiplier, 2.0);
    }

    #[test]
    fn test_api_client_creation() {
        let client = ApiClient::new(
            "test_key".to_string(),
            "https://api.anthropic.com".to_string(),
        );
        assert_eq!(client.api_key, "test_key");
        assert_eq!(client.api_url, "https://api.anthropic.com");
    }
}
