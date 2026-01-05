# Rust Claude Code 使用指南

## 快速开始

### 1. 设置 API Key

```bash
export ANTHROPIC_API_KEY=your_api_key_here
```

### 2. 构建项目

```bash
# 使用构建脚本
./build.sh

# 或者直接使用 cargo
cargo build --release
```

### 3. 运行程序

#### 交互模式
```bash
cargo run --release
# 或
./target/release/rust-claude-code
```

#### 单次执行模式
```bash
# 列出文件
cargo run --release -- --prompt "列出当前目录的 Rust 文件"

# 读取文件
cargo run --release -- --prompt "读取 Cargo.toml 文件"

# 执行命令
cargo run --release -- --prompt "运行 git status"

# 写入文件
cargo run --release -- --prompt "创建一个名为 test.txt 的文件，内容是 Hello World"
```

## 功能特性

### 支持的工具

1. **read_file** - 读取文件内容
2. **write_file** - 写入文件内容
3. **execute_command** - 执行 shell 命令
4. **list_files** - 使用 glob 模式列出文件

### 命令行参数

```bash
rust-claude-code [OPTIONS]

OPTIONS:
    -a, --api-key <API_KEY>          Anthropic API 密钥
    -m, --max-turns <MAX_TURNS>      最大对话轮数 [默认: 10]
    -p, --prompt <PROMPT>            单次执行模式
    -h, --help                       显示帮助信息
    -V, --version                    显示版本信息
```

## 示例对话

```
You: 列出当前目录的所有 Rust 源文件
Tool: list_files
Claude: 我找到了以下 Rust 文件:
- src/main.rs

You: 读取 src/main.rs 的内容
Tool: read_file
Claude: [显示文件内容...]

You: 在 src/ 目录下创建一个新的模块文件 utils.rs
Tool: write_file
Claude: 已成功创建文件: src/utils.rs
```

## 故障排除

### Cargo 构建失败

如果遇到 `NSInvalidArgumentException` 错误:

1. **检查网络代理设置**
```bash
unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY ALL_PROXY
```

2. **尝试重新安装 Rust**
```bash
rustup update
rustup self reinstall
```

3. **使用官方安装脚本**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

4. **检查系统配置**
```bash
# macOS 系统偏好设置 -> 网络 -> 高级 -> 代理
# 确保没有冲突的代理设置
```

### API Key 未设置

```bash
# 临时设置
export ANTHROPIC_API_KEY=sk-ant-...

# 永久设置（添加到 ~/.zshrc 或 ~/.bashrc）
echo 'export ANTHROPIC_API_KEY=sk-ant-...' >> ~/.zshrc
source ~/.zshrc
```

## 项目结构

```
rust-claude-code/
├── Cargo.toml          # 项目配置
├── src/
│   └── main.rs         # 完整实现（单文件）
├── build.sh            # 构建脚本
├── README.md           # 项目说明
└── USAGE.md            # 使用指南（本文件）
```

## 技术栈

- **Rust** - 系统编程语言
- **Tokio** - 异步运行时
- **Clap** - 命令行参数解析
- **Reqwest** - HTTP 客户端
- **Serde** - 序列化/反序列化
- **Console** - 终端样式
- **Dialoguer** - 交互式输入

## 开发

```bash
# 运行测试
cargo test

# 检查代码
cargo check

# 格式化代码
cargo fmt

# 生成文档
cargo doc --open
```

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
