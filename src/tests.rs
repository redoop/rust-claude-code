#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::security;
    use mockito::mock;
    use serde_json::json;
    use std::fs;
    use std::fs;
    use std::io::Write;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};
    use tempfile::{NamedTempFile, TempDir};

    // 配置测试
    mod config_tests {
        use super::*;

        #[test]
        fn test_default_user_settings() {
            let settings = config::UserSettings::default();
            assert_eq!(settings.theme, "default");
            assert_eq!(settings.auto_save, false);
            assert_eq!(settings.ai_enabled, true);
            assert_eq!(settings.confidence_threshold, 0.8);
            assert_eq!(settings.enabled_plugins.len(), 1);
            assert_eq!(
                settings.enabled_plugins[0],
                "rust-analyzer-lsp@claude-plugins-official"
            );
        }

        #[test]
        fn test_default_local_settings() {
            let settings = config::LocalSettings::default();
            assert!(settings.anthropic_auth_token.is_none());
        }

        #[test]
        fn test_settings_serialization() {
            let settings = config::UserSettings::default();
            let json_str = serde_json::to_string(&settings).unwrap();
            let parsed: config::UserSettings = serde_json::from_str(&json_str).unwrap();
            assert_eq!(settings.theme, parsed.theme);
            assert_eq!(settings.auto_save, parsed.auto_save);
        }

        #[test]
        fn test_config_load_with_missing_dir() {
            let temp_dir = TempDir::new().unwrap();
            let original_dir = std::env::current_dir().unwrap();

            std::env::set_current_dir(&temp_dir).unwrap();

            // 这应该创建默认配置
            let result = config::Config::load();
            assert!(result.is_ok());

            // 检查是否创建了配置文件
            let settings_path = temp_dir.path().join(".claude/settings.json");
            assert!(settings_path.exists());

            std::env::set_current_dir(original_dir).unwrap();
        }
    }

    // 安全测试
    mod security_tests {
        use super::*;

        #[test]
        fn test_validate_safe_file_paths() {
            // 有效路径
            assert!(security::InputValidator::validate_file_path("/tmp/test.txt").is_ok());
            assert!(security::InputValidator::validate_file_path("/home/user/file.rs").is_ok());

            // 无效路径
            assert!(security::InputValidator::validate_file_path("").is_err());
            assert!(security::InputValidator::validate_file_path("../etc/passwd").is_err());
            assert!(security::InputValidator::validate_file_path("/path/../../../etc").is_err());
            assert!(security::InputValidator::validate_file_path("/path\0with\0null").is_err());
        }

        #[test]
        fn test_validate_safe_commands() {
            // 有效命令
            assert!(security::InputValidator::validate_command("ls -la").is_ok());
            assert!(security::InputValidator::validate_command("cargo build").is_ok());
            assert!(security::InputValidator::validate_command("git status").is_ok());

            // 危险命令
            assert!(security::InputValidator::validate_command("rm -rf /").is_err());
            assert!(security::InputValidator::validate_command("sudo rm -rf /").is_err());
            assert!(security::InputValidator::validate_command("format c:").is_err());
            assert!(security::InputValidator::validate_command("").is_err());
            assert!(security::InputValidator::validate_command("shutdown now").is_err());
        }

        #[test]
        fn test_validate_glob_patterns() {
            // 有效模式
            assert!(security::InputValidator::validate_glob_pattern("*.rs").is_ok());
            assert!(security::InputValidator::validate_glob_pattern("src/**/*.rs").is_ok());

            // 无效模式
            assert!(security::InputValidator::validate_glob_pattern("").is_err());
            assert!(security::InputValidator::validate_glob_pattern("../**/*").is_err());
            assert!(
                security::InputValidator::validate_glob_pattern("pattern\0with\0null").is_err()
            );
        }

        #[test]
        fn test_validate_api_keys() {
            // 有效 API key
            assert!(security::InputValidator::validate_api_key("sk-ant-test123456789").is_ok());

            // 无效 API key
            assert!(security::InputValidator::validate_api_key("").is_err());
            assert!(security::InputValidator::validate_api_key("invalid-key").is_err());
            assert!(security::InputValidator::validate_api_key("sk-ant-").is_err());
        }

        #[tokio::test]
        async fn test_safe_read_file() {
            let temp_file = NamedTempFile::new().unwrap();
            let file_path = temp_file.path().to_str().unwrap();

            // 写入测试内容
            fs::write(file_path, "Hello, World!").unwrap();

            let input = json!({"file_path": file_path});
            let result = security::SafeToolExecutor::safe_read_file(&input).await;

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "Hello, World!");
        }

        #[tokio::test]
        async fn test_safe_write_file() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt").to_str().unwrap();

            let input = json!({
                "file_path": file_path,
                "content": "Test content"
            });

            let result = security::SafeToolExecutor::safe_write_file(&input).await;

            assert!(result.is_ok());
            assert!(fs::metadata(file_path).is_ok());

            let content = fs::read_to_string(file_path).unwrap();
            assert_eq!(content, "Test content");
        }

        #[tokio::test]
        async fn test_safe_execute_command() {
            let input = json!({"command": "echo 'Hello, Test!'"});
            let result = security::SafeToolExecutor::safe_execute_command(&input).await;

            assert!(result.is_ok());
            assert!(result.unwrap().contains("Hello, Test!"));
        }

        #[tokio::test]
        async fn test_safe_list_files() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path().to_str().unwrap();

            // 创建测试文件
            fs::write(temp_dir.path().join("test1.rs"), "content1").unwrap();
            fs::write(temp_dir.path().join("test2.rs"), "content2").unwrap();
            fs::write(temp_dir.path().join("test.txt"), "content3").unwrap();

            let input = json!({
                "pattern": "*.rs",
                "path": dir_path
            });

            let result = security::SafeToolExecutor::safe_list_files(&input).await;

            assert!(result.is_ok());
            let files = result.unwrap();
            assert!(files.contains("test1.rs"));
            assert!(files.contains("test2.rs"));
            assert!(!files.contains("test.txt"));
        }
    }

    // 错误处理测试
    mod error_tests {
        use super::*;

        #[test]
        fn test_retry_config_default() {
            let config = error::RetryConfig::default();
            assert_eq!(config.max_retries, 3);
            assert_eq!(config.initial_delay, std::time::Duration::from_millis(1000));
            assert_eq!(config.max_delay, std::time::Duration::from_secs(30));
            assert_eq!(config.multiplier, 2.0);
        }

        #[test]
        fn test_api_client_creation() {
            let client = error::ApiClient::new(
                "test_key".to_string(),
                "https://api.anthropic.com".to_string(),
            );
            assert_eq!(client.api_key, "test_key");
            assert_eq!(client.api_url, "https://api.anthropic.com");
        }

        #[test]
        fn test_api_error_display() {
            let auth_error = error::ApiError::Authentication;
            assert_eq!(
                auth_error.to_string(),
                "Authentication failed: invalid API key"
            );

            let rate_limit_error = error::ApiError::RateLimit(60);
            assert_eq!(
                rate_limit_error.to_string(),
                "Rate limit exceeded, retry after 60 seconds"
            );

            let timeout_error = error::ApiError::Timeout(30);
            assert_eq!(timeout_error.to_string(), "Timeout after 30 seconds");
        }

        #[tokio::test]
        async fn test_api_call_with_mock() {
            let mock_response = json!({
                "id": "msg_test",
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "Hello, World!"
                    }
                ]
            });

            let _mock = mock("POST", "/v1/messages")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(serde_json::to_string(&mock_response).unwrap())
                .create();

            let client = error::ApiClient::new(
                "test_key".to_string(),
                format!("{}/v1/messages", mockito::server_url()),
            );

            let messages = json!([
                {
                    "role": "user",
                    "content": "Hello"
                }
            ]);

            let result = client.call_claude_with_retry(&messages, false).await;
            assert!(result.is_ok());
        }
    }

    // 集成测试
    mod integration_tests {
        use super::*;

        #[tokio::test]
        async fn test_full_workflow() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt").to_str().unwrap();

            // 1. 写入文件
            let write_input = json!({
                "file_path": file_path,
                "content": "Initial content"
            });

            let write_result = security::SafeToolExecutor::safe_write_file(&write_input).await;
            assert!(write_result.is_ok());

            // 2. 读取文件
            let read_input = json!({"file_path": file_path});
            let read_result = security::SafeToolExecutor::safe_read_file(&read_input).await;
            assert!(read_result.is_ok());
            assert_eq!(read_result.unwrap(), "Initial content");

            // 3. 列出文件
            let list_input = json!({
                "pattern": "*.txt",
                "path": temp_dir.path().to_str().unwrap()
            });

            let list_result = security::SafeToolExecutor::safe_list_files(&list_input).await;
            assert!(list_result.is_ok());
            assert!(list_result.unwrap().contains("test.txt"));
        }

        #[test]
        fn test_error_recovery() {
            // 测试各种错误情况的处理
            let invalid_path = "";
            let result = security::InputValidator::validate_file_path(invalid_path);
            assert!(result.is_err());

            let invalid_command = "rm -rf /";
            let result = security::InputValidator::validate_command(invalid_command);
            assert!(result.is_err());

            let invalid_api_key = "invalid";
            let result = security::InputValidator::validate_api_key(invalid_api_key);
            assert!(result.is_err());
        }
    }

    // 性能测试
    mod performance_tests {
        use super::*;
        use std::time::Instant;

        #[tokio::test]
        async fn test_large_file_handling() {
            let temp_file = NamedTempFile::new().unwrap();
            let file_path = temp_file.path().to_str().unwrap();

            // 创建大文件 (1MB)
            let large_content = "A".repeat(1024 * 1024);
            fs::write(file_path, &large_content).unwrap();

            let start = Instant::now();
            let input = json!({"file_path": file_path});
            let result = security::SafeToolExecutor::safe_read_file(&input).await;
            let duration = start.elapsed();

            assert!(result.is_ok());
            assert_eq!(result.unwrap().len(), 1024 * 1024);

            // 性能断言：应该在合理时间内完成
            assert!(duration.as_secs() < 5);
        }

        #[test]
        fn test_validation_performance() {
            let start = Instant::now();

            for i in 0..1000 {
                let path = format!("/tmp/test_{}.txt", i);
                let _ = security::InputValidator::validate_file_path(&path);
            }

            let duration = start.elapsed();

            // 1000 次验证应该在 1 秒内完成
            assert!(duration.as_millis() < 1000);
        }
    }
}
