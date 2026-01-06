use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use tokio::fs as async_fs;
use tokio::io::AsyncBufReadExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn};

/// 大文件处理配置
#[derive(Debug, Clone)]
pub struct FileProcessingConfig {
    /// 大文件阈值 (字节)
    pub large_file_threshold: usize,
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 分块大小
    pub chunk_size: usize,
    /// 最大读取时间 (秒)
    pub max_read_time: u64,
}

impl Default for FileProcessingConfig {
    fn default() -> Self {
        Self {
            large_file_threshold: 1024 * 1024, // 1MB
            buffer_size: 64 * 1024,            // 64KB
            chunk_size: 8192,                  // 8KB
            max_read_time: 30,                 // 30秒
        }
    }
}

/// 高性能文件处理器
pub struct FileProcessor {
    config: FileProcessingConfig,
}

impl FileProcessor {
    pub fn new() -> Self {
        Self {
            config: FileProcessingConfig::default(),
        }
    }

    pub fn with_config(config: FileProcessingConfig) -> Self {
        Self { config }
    }

    /// 高效读取文件内容
    pub async fn read_file_efficiently(&self, file_path: &Path) -> Result<String> {
        let metadata = async_fs::metadata(file_path)
            .await
            .with_context(|| format!("Failed to get metadata for: {}", file_path.display()))?;

        let file_size = metadata.len() as usize;

        // 根据文件大小选择不同的读取策略
        match file_size {
            0 => Ok(String::new()),
            size if size <= self.config.large_file_threshold => {
                self.read_small_file(file_path).await
            }
            size if size <= 50 * 1024 * 1024 => {
                // 50MB
                self.read_medium_file(file_path).await
            }
            _ => {
                warn!(
                    "File is very large ({} bytes), reading in chunks",
                    file_size
                );
                self.read_large_file(file_path).await
            }
        }
    }

    /// 读取小文件 (使用标准读取)
    async fn read_small_file(&self, file_path: &Path) -> Result<String> {
        info!("Reading small file: {}", file_path.display());
        let content = async_fs::read_to_string(file_path)
            .await
            .with_context(|| format!("Failed to read small file: {}", file_path.display()))?;
        Ok(content)
    }

    /// 读取中等大小文件 (使用缓冲读取)
    async fn read_medium_file(&self, file_path: &Path) -> Result<String> {
        info!(
            "Reading medium file with buffering: {}",
            file_path.display()
        );

        let mut file = async_fs::File::open(file_path)
            .await
            .with_context(|| format!("Failed to open medium file: {}", file_path.display()))?;

        let mut buffer = Vec::with_capacity(self.config.buffer_size);
        file.read_to_end(&mut buffer)
            .await
            .with_context(|| format!("Failed to read medium file: {}", file_path.display()))?;

        let content = String::from_utf8(buffer)
            .with_context(|| format!("File contains invalid UTF-8: {}", file_path.display()))?;

        Ok(content)
    }

    /// 读取大文件 (使用分块读取，并限制读取量)
    async fn read_large_file(&self, file_path: &Path) -> Result<String> {
        info!("Reading large file in chunks: {}", file_path.display());

        let mut file = async_fs::File::open(file_path)
            .await
            .with_context(|| format!("Failed to open large file: {}", file_path.display()))?;

        let mut buffer = Vec::new();
        let mut chunk = vec![0u8; self.config.chunk_size];
        let mut total_read = 0;
        let max_content = 10 * 1024 * 1024; // 10MB 最大内容

        loop {
            let bytes_read = file
                .read(&mut chunk)
                .await
                .with_context(|| format!("Failed to read chunk from: {}", file_path.display()))?;

            if bytes_read == 0 {
                break;
            }

            buffer.extend_from_slice(&chunk[..bytes_read]);
            total_read += bytes_read;

            // 限制读取的总量
            if total_read >= max_content {
                warn!(
                    "File truncated at {} bytes (original size: {})",
                    max_content, total_read
                );
                break;
            }

            // 添加超时保护
            if total_read % (1024 * 1024) == 0 {
                // 每读取 1MB 检查一次
                info!("Read {} MB so far", total_read / (1024 * 1024));
            }
        }

        let content = String::from_utf8(buffer)
            .with_context(|| format!("File contains invalid UTF-8: {}", file_path.display()))?;

        Ok(content)
    }

    /// 高效写入文件
    pub async fn write_file_efficiently(&self, file_path: &Path, content: &str) -> Result<()> {
        let content_size = content.len();

        info!("Writing {} bytes to: {}", content_size, file_path.display());

        // 根据内容大小选择写入策略
        if content_size <= self.config.large_file_threshold {
            self.write_small_file(file_path, content).await
        } else {
            self.write_large_file(file_path, content).await
        }
    }

    /// 写入小文件
    async fn write_small_file(&self, file_path: &Path, content: &str) -> Result<()> {
        async_fs::write(file_path, content)
            .await
            .with_context(|| format!("Failed to write small file: {}", file_path.display()))?;
        Ok(())
    }

    /// 写入大文件 (使用缓冲写入)
    async fn write_large_file(&self, file_path: &Path, content: &str) -> Result<()> {
        // 确保父目录存在
        if let Some(parent) = file_path.parent() {
            async_fs::create_dir_all(parent).await.with_context(|| {
                format!("Failed to create parent directory: {}", parent.display())
            })?;
        }

        let mut file = async_fs::File::create(file_path)
            .await
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;

        // 分块写入
        let mut bytes_written = 0;
        let chunks = content.as_bytes().chunks(self.config.chunk_size);

        for chunk in chunks {
            file.write_all(chunk)
                .await
                .with_context(|| format!("Failed to write chunk to: {}", file_path.display()))?;
            bytes_written += chunk.len();

            // 添加进度日志
            if bytes_written % (1024 * 1024) == 0 {
                info!("Written {} MB so far", bytes_written / (1024 * 1024));
            }
        }

        file.flush()
            .await
            .with_context(|| format!("Failed to flush file: {}", file_path.display()))?;

        Ok(())
    }

    /// 流式处理文件行
    pub async fn process_file_lines<F>(&self, file_path: &Path, mut processor: F) -> Result<()>
    where
        F: FnMut(&str) -> Result<()>,
    {
        info!("Processing file lines: {}", file_path.display());

        let file = async_fs::File::open(file_path).await.with_context(|| {
            format!(
                "Failed to open file for line processing: {}",
                file_path.display()
            )
        })?;

        let reader = tokio::io::BufReader::new(file);
        let mut lines = reader.lines();

        let mut line_count = 0;
        while let Some(line_result) = lines.next_line().await.transpose() {
            let line = line_result
                .with_context(|| format!("Failed to read line from: {}", file_path.display()))?;
            processor(&line)?;
            line_count += 1;

            // 处理大文件的进度报告
            if line_count % 10000 == 0 {
                info!("Processed {} lines", line_count);
            }
        }

        info!("Processed {} lines total", line_count);
        Ok(())
    }

    /// 同步版本的文件读取 (用于不支持异步的上下文)
    pub fn read_file_sync(&self, file_path: &Path) -> Result<String> {
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open file: {}", file_path.display()))?;

        let metadata = file
            .metadata()
            .with_context(|| format!("Failed to get metadata: {}", file_path.display()))?;

        let file_size = metadata.len() as usize;

        if file_size > 50 * 1024 * 1024 {
            // 50MB
            return Err(anyhow::anyhow!(
                "File too large for sync reading: {} bytes",
                file_size
            ));
        }

        let mut reader = BufReader::with_capacity(self.config.buffer_size, file);
        let mut content = String::with_capacity(file_size);

        reader
            .read_to_string(&mut content)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        Ok(content)
    }

    /// 同步版本的文件写入
    pub fn write_file_sync(&self, file_path: &Path, content: &str) -> Result<()> {
        // 确保父目录存在
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create parent directory: {}", parent.display())
            })?;
        }

        let file = File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path.display()))?;

        let mut writer = io::BufWriter::with_capacity(self.config.buffer_size, file);

        writer
            .write_all(content.as_bytes())
            .with_context(|| format!("Failed to write to file: {}", file_path.display()))?;

        writer
            .flush()
            .with_context(|| format!("Failed to flush file: {}", file_path.display()))?;

        Ok(())
    }

    /// 获取文件信息
    pub async fn get_file_info(&self, file_path: &Path) -> Result<FileInfo> {
        let metadata = async_fs::metadata(file_path)
            .await
            .with_context(|| format!("Failed to get metadata: {}", file_path.display()))?;

        Ok(FileInfo {
            size: metadata.len(),
            is_file: metadata.is_file(),
            is_directory: metadata.is_dir(),
            is_symlink: metadata.file_type().is_symlink(),
            path: file_path.to_path_buf(),
        })
    }
}

/// 文件信息结构
#[derive(Debug)]
pub struct FileInfo {
    pub size: u64,
    pub is_file: bool,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub path: std::path::PathBuf,
}

impl FileInfo {
    /// 格式化文件大小
    pub fn format_size(&self) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = self.size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", self.size, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_small_file_processing() {
        let processor = FileProcessor::new();
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        // 写入小文件内容
        let content = "Hello, World!";
        std::fs::write(file_path, content).unwrap();

        // 读取文件
        let read_content = processor.read_file_efficiently(file_path).await.unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_large_file_processing() {
        let config = FileProcessingConfig {
            large_file_threshold: 10, // 很小的阈值，强制使用大文件处理
            ..Default::default()
        };
        let processor = FileProcessor::with_config(config);

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        // 写入较大内容
        let content = "A".repeat(100);
        std::fs::write(file_path, &content).unwrap();

        // 读取文件
        let read_content = processor.read_file_efficiently(file_path).await.unwrap();
        assert_eq!(read_content.len(), 100);
    }

    #[tokio::test]
    async fn test_file_info() {
        let processor = FileProcessor::new();
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        let file_info = processor.get_file_info(file_path).await.unwrap();

        assert!(file_info.is_file);
        assert!(!file_info.is_directory);
        assert_eq!(file_info.path, file_path);

        // 测试格式化
        let formatted = file_info.format_size();
        assert!(formatted.ends_with("B"));
    }

    #[test]
    fn test_sync_operations() {
        let processor = FileProcessor::new();
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        // 测试同步写入
        let content = "Test content for sync operations";
        processor.write_file_sync(file_path, content).unwrap();

        // 测试同步读取
        let read_content = processor.read_file_sync(file_path).unwrap();
        assert_eq!(read_content, content);
    }
}
