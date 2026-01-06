use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// 输入验证器
pub struct InputValidator;

impl InputValidator {
    /// 验证文件路径是否安全
    pub fn validate_file_path(file_path: &str) -> Result<PathBuf> {
        if file_path.is_empty() {
            return Err(anyhow!("File path cannot be empty"));
        }

        // 检查路径遍历攻击
        if file_path.contains("..") {
            return Err(anyhow!("Path traversal detected: {}", file_path));
        }

        // 检查危险字符
        let dangerous_chars = ['\0', '<', '>', '|', '"', '\n', '\r'];
        for ch in dangerous_chars {
            if file_path.contains(ch) {
                return Err(anyhow!(
                    "Dangerous character '{}' in path: {}",
                    ch,
                    file_path
                ));
            }
        }

        let path = PathBuf::from(file_path);

        // 确保路径是绝对的
        if !path.is_absolute() {
            return Err(anyhow!("Only absolute paths are allowed: {}", file_path));
        }

        // 检查路径是否在允许的目录内
        Self::check_allowed_directory(&path)?;

        Ok(path)
    }

    /// 验证命令是否安全
    pub fn validate_command(command: &str) -> Result<String> {
        if command.is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }

        // 检查危险命令
        let dangerous_commands = [
            "rm -rf /", "sudo rm", "format", "del /f", "shutdown", "reboot", "halt", "poweroff",
            "mkfs", "fdisk", "dd if=",
        ];

        let lower_command = command.to_lowercase();
        for dangerous in &dangerous_commands {
            if lower_command.contains(dangerous) {
                return Err(anyhow!("Dangerous command detected: {}", command));
            }
        }

        // 检查管道和重定向（限制使用）
        let pipe_count = command.matches('|').count();
        let redirect_count = command.matches('>').count() + command.matches('<').count();

        if pipe_count > 2 {
            return Err(anyhow!("Too many pipes in command: {}", command));
        }

        if redirect_count > 2 {
            return Err(anyhow!("Too many redirects in command: {}", command));
        }

        // 检查命令长度
        if command.len() > 1000 {
            return Err(anyhow!("Command too long: {} characters", command.len()));
        }

        Ok(command.to_string())
    }

    /// 验证 glob 模式
    pub fn validate_glob_pattern(pattern: &str) -> Result<String> {
        if pattern.is_empty() {
            return Err(anyhow!("Pattern cannot be empty"));
        }

        // 检查路径遍历
        if pattern.contains("..") {
            return Err(anyhow!("Path traversal detected in pattern: {}", pattern));
        }

        // 检查危险字符
        if pattern.contains('\0') {
            return Err(anyhow!("Null character in pattern: {}", pattern));
        }

        // 检查模式复杂度
        let star_count = pattern.matches('*').count();
        if star_count > 10 {
            return Err(anyhow!(
                "Pattern too complex (too many wildcards): {}",
                pattern
            ));
        }

        Ok(pattern.to_string())
    }

    /// 验证 API 密钥格式
    pub fn validate_api_key(api_key: &str) -> Result<String> {
        if api_key.is_empty() {
            return Err(anyhow!("API key cannot be empty"));
        }

        // Anthropic API key 通常以 "sk-ant-" 开头
        if !api_key.starts_with("sk-ant-") {
            return Err(anyhow!(
                "Invalid API key format (should start with 'sk-ant-')"
            ));
        }

        // 检查长度（通常 40-50 字符）
        if api_key.len() < 30 || api_key.len() > 100 {
            return Err(anyhow!("API key length invalid"));
        }

        Ok(api_key.to_string())
    }

    /// 检查路径是否在允许的目录内
    fn check_allowed_directory(path: &Path) -> Result<()> {
        // 获取当前工作目录
        let current_dir = env::current_dir().context("Failed to get current directory")?;

        // 获取用户主目录
        let home_dir = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"));

        // 定义允许的目录列表
        let allowed_dirs = vec![
            current_dir,
            home_dir,
            PathBuf::from("/tmp"),
            PathBuf::from("/var/tmp"),
        ];

        // 检查路径是否在允许的目录内
        for allowed_dir in allowed_dirs {
            if let Ok(relative) = path.strip_prefix(&allowed_dir) {
                // 确保相对路径不包含 ".."
                if !relative.starts_with("..") {
                    return Ok(());
                }
            }
        }

        Err(anyhow!("Path not in allowed directory: {}", path.display()))
    }

    /// 清理和规范化路径
    pub fn sanitize_path(path: &Path) -> Result<PathBuf> {
        let canonical = fs::canonicalize(path).context("Failed to canonicalize path")?;

        Ok(canonical)
    }

    /// 检查文件权限
    pub fn check_file_permissions(path: &Path) -> Result<()> {
        if !path.exists() {
            return Ok(()); // 文件不存在是允许的
        }

        let metadata = fs::metadata(path).context("Failed to get file metadata")?;

        // 检查是否为符号链接
        if metadata.file_type().is_symlink() {
            return Err(anyhow!(
                "Symbolic links are not allowed: {}",
                path.display()
            ));
        }

        // 检查文件大小（限制为 100MB）
        if metadata.is_file() {
            if metadata.len() > 100 * 1024 * 1024 {
                return Err(anyhow!("File too large: {} bytes", metadata.len()));
            }
        }

        Ok(())
    }
}

/// 安全工具执行器
pub struct SafeToolExecutor;

impl SafeToolExecutor {
    /// 安全地执行工具调用
    pub async fn execute_tool_safely(name: &str, input: &serde_json::Value) -> Result<String> {
        match name {
            "read_file" => Self::safe_read_file(input).await,
            "write_file" => Self::safe_write_file(input).await,
            "execute_command" => Self::safe_execute_command(input).await,
            "list_files" => Self::safe_list_files(input).await,
            _ => Err(anyhow!("Unknown tool: {}", name)),
        }
    }

    /// 安全读取文件
    async fn safe_read_file(input: &serde_json::Value) -> Result<String> {
        let file_path = input["file_path"].as_str().context("Missing file_path")?;

        // 验证路径
        let validated_path = InputValidator::validate_file_path(file_path)?;

        // 检查权限
        InputValidator::check_file_permissions(&validated_path)?;

        // 规范化路径
        let safe_path = InputValidator::sanitize_path(&validated_path)?;

        // 读取文件
        let content = fs::read_to_string(&safe_path)
            .with_context(|| format!("Failed to read file: {}", safe_path.display()))?;

        // 检查内容大小并优化大文件处理
        if content.len() > 10 * 1024 * 1024 {
            // 10MB
            return Err(anyhow!("File content too large: {} bytes", content.len()));
        }

        // 对于中等大小的文件，优化处理
        if content.len() > 1024 * 1024 {
            // 1MB
            info!("Processing large file: {} bytes", content.len());
            // 可以在这里添加流式处理或分块处理逻辑
        }

        Ok(content)
    }

    /// 安全写入文件
    async fn safe_write_file(input: &serde_json::Value) -> Result<String> {
        let file_path = input["file_path"].as_str().context("Missing file_path")?;
        let content = input["content"].as_str().context("Missing content")?;

        // 验证路径
        let validated_path = InputValidator::validate_file_path(file_path)?;

        // 检查内容大小
        if content.len() > 50 * 1024 * 1024 {
            // 50MB
            return Err(anyhow!("Content too large: {} bytes", content.len()));
        }

        // 确保父目录存在
        if let Some(parent) = validated_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        // 写入文件
        fs::write(&validated_path, content)
            .with_context(|| format!("Failed to write file: {}", validated_path.display()))?;

        Ok(format!(
            "Successfully wrote to file: {}",
            validated_path.display()
        ))
    }

    /// 安全执行命令
    async fn safe_execute_command(input: &serde_json::Value) -> Result<String> {
        let command = input["command"].as_str().context("Missing command")?;

        // 验证命令
        let safe_command = InputValidator::validate_command(command)?;

        println!("\n{}", console::style("Executing:").cyan());
        println!("  {}", console::style(&safe_command).yellow());

        // 执行命令
        let output = if cfg!(target_os = "windows") {
            std::process::Command::new("cmd")
                .args(["/C", &safe_command])
                .output()?
        } else {
            std::process::Command::new("sh")
                .args(["-c", &safe_command])
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

        // 检查命令是否成功
        if !output.status.success() {
            warn!("Command failed with exit code: {}", output.status);
        }

        Ok(result)
    }

    /// 安全列出文件
    async fn safe_list_files(input: &serde_json::Value) -> Result<String> {
        let pattern = input["pattern"].as_str().context("Missing pattern")?;
        let base_path = input["path"].as_str().unwrap_or(".");

        // 验证模式
        let safe_pattern = InputValidator::validate_glob_pattern(pattern)?;

        // 验证基础路径
        let validated_base = InputValidator::validate_file_path(base_path)?;

        use glob::glob;

        let full_pattern = if safe_pattern.starts_with('/') {
            safe_pattern.clone()
        } else {
            format!("{}/{}", validated_base.display(), safe_pattern)
        };

        let mut files = Vec::new();
        let mut file_count = 0;

        for entry in glob(&full_pattern)
            .with_context(|| format!("Failed to read glob pattern: {}", full_pattern))?
        {
            match entry {
                Ok(path) => {
                    // 限制结果数量
                    if file_count >= 1000 {
                        warn!("Too many files found, limiting to 1000");
                        break;
                    }

                    if let Some(path_str) = path.to_str() {
                        files.push(path_str.to_string());
                        file_count += 1;
                    }
                }
                Err(e) => {
                    warn!("Error reading entry: {:?}", e);
                }
            }
        }

        if files.is_empty() {
            Ok("(no files found)".to_string())
        } else {
            Ok(files.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_safe_path() {
        assert!(InputValidator::validate_file_path("/tmp/test.txt").is_ok());
        assert!(InputValidator::validate_file_path("../etc/passwd").is_err());
        assert!(InputValidator::validate_file_path("").is_err());
    }

    #[test]
    fn test_validate_safe_command() {
        assert!(InputValidator::validate_command("ls -la").is_ok());
        assert!(InputValidator::validate_command("rm -rf /").is_err());
        assert!(InputValidator::validate_command("").is_err());
    }

    #[test]
    fn test_validate_api_key() {
        assert!(InputValidator::validate_api_key("sk-ant-test123").is_ok());
        assert!(InputValidator::validate_api_key("invalid-key").is_err());
        assert!(InputValidator::validate_api_key("").is_err());
    }
}
