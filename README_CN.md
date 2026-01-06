# 🦀 Rust Claude Code

[![Rust](https://img.shields.io/badge/rust-2021%20Edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/yourusername/rust-claude-code)

一个高性能、安全的 Rust 实现的 Claude AI 代码助手命令行工具，通过 Anthropic Claude API 提供智能代码辅助功能。

[English Documentation](README.md) | 中文文档

## ✨ 特性

- 🚀 **高性能** - 基于 Rust 的高性能实现，快速响应
- 🔒 **安全可靠** - 类型安全、内存安全和输入验证保证
- 🛡️ **安全增强** - 路径验证、命令过滤、防注入攻击
- 🛠️ **工具支持** - 支持文件操作、命令执行等功能
- ⚙️ **配置管理** - 灵活的配置系统，支持多种配置源
- 🎨 **彩色输出** - 美观的终端彩色输出
- 🔄 **交互模式** - 支持交互式对话和单次提示模式
- 🌐 **跨平台** - 支持 Windows、macOS 和 Linux

## 📋 功能

### 核心功能

- ✅ **AI 对话系统**
  - 交互式多轮对话
  - 单次提示模式
  - 对话历史管理

- ✅ **文件系统操作**
  - 文件读取
  - 文件写入（自动创建目录）
  - Glob 模式文件搜索

- ✅ **命令执行**
  - 跨平台命令支持（Windows cmd / Unix sh）
  - 实时输出显示
  - 错误捕获

- ✅ **配置系统**
  - 用户配置文件 (`.claude/settings.json`)
  - 本地配置文件 (`.claude/settings.local.json`)
  - 环境变量支持
  - 命令行参数覆盖

- ✅ **安全增强**
  - 文件路径验证和规范化
  - 危险命令检测和过滤
  - API 密钥格式验证
  - 输入大小限制
  - 文件权限检查

- ✅ **错误处理**
  - 指数退避重试机制
  - 详细错误分类
  - 网络超时保护
  - 结构化日志记录

- ✅ **测试覆盖**
  - 单元测试覆盖核心功能
  - 集成测试验证工作流
  - 安全测试确保防护有效性
  - 性能测试保证响应时间

## 🚀 快速开始

### 环境要求

- Rust 2021 Edition 或更高版本
- Anthropic API 密钥

### 安装

1. 克隆仓库
```bash
git clone https://github.com/yourusername/rust-claude-code.git
cd rust-claude-code
```

2. 构建项目
```bash
cargo build --release
```

3. 配置 API 密钥

方式一：环境变量
```bash
export ANTHROPIC_API_KEY=your_api_key_here
```

方式二：配置文件
```bash
mkdir -p .claude
cat > .claude/settings.json << EOF
{
  "anthropic_api_key": "your_api_key_here",
  "theme": "default",
  "ai_enabled": true
}
EOF
```

### 使用方法

#### 交互模式

```bash
# 使用环境变量中的 API key
cargo run --release

# 使用命令行参数指定 API key
cargo run --release -- --api-key your_api_key

# 设置最大对话轮数
cargo run --release -- --max-turns 20
```

#### 单次提示模式

```bash
# 直接执行单个提示
cargo run --release -- --prompt "帮我写一个 Hello World 程序"

# 结合 API key
cargo run --release -- --api-key your_key --prompt "解释这段代码"
```

#### 自定义 API 端点

```bash
# 使用自定义 API URL（支持代理）
cargo run --release -- --api-url https://your-proxy.com/v1/messages

# 通过环境变量设置
export ANTHROPIC_BASE_URL=https://your-proxy.com/v1/messages
cargo run --release
```

#### 配置管理

```bash
# 查看配置文件路径
cargo run --release -- --show-config
```

## ⚙️ 配置

### 配置文件结构

#### 用户配置 (`.claude/settings.json`)

```json
{
  "theme": "default",
  "auto_save": false,
  "ai_enabled": true,
  "anthropic_api_key": "your_api_key_here",
  "api_base_url": "https://api.anthropic.com",
  "confidence_threshold": 0.8,
  "enabled_plugins": [
    "rust-analyzer-lsp@claude-plugins-official"
  ]
}
```

#### 本地配置 (`.claude/settings.local.json`)

此文件不应提交到版本控制系统，用于本地覆盖配置：

```json
{
  "anthropic_auth_token": "your_local_token",
  "custom_settings": {}
}
```

### 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `ANTHROPIC_API_KEY` | Anthropic API 密钥 | - |
| `ANTHROPIC_AUTH_TOKEN` | API 认证令牌（同上） | - |
| `ANTHROPIC_BASE_URL` | API 基础 URL | `https://api.anthropic.com` |
| `API_TIMEOUT_MS` | API 超时时间（毫秒） | `120000` |
| `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` | 禁用非必要流量 | `false` |

### 命令行参数

```
rust-claude-code [OPTIONS]

选项:
  -k, --api-key <API_KEY>           Anthropic API 密钥（覆盖配置）
  -m, --max-turns <MAX_TURNS>      最大对话轮数 [默认: 10]
  -p, --prompt <PROMPT>            单次提示模式
  -u, --api-url <API_URL>          API 基础 URL（覆盖配置）
  -t, --timeout <TIMEOUT>          超时时间（秒）
      --show-config                显示配置文件路径
  -h, --help                       显示帮助信息
  -V, --version                    显示版本信息
```

## 🛠️ 开发

### 项目结构

```
rust-claude-code/
├── src/
│   ├── main.rs          # 主程序入口
│   ├── config.rs        # 配置管理模块
│   ├── error.rs         # 错误处理和重试机制
│   ├── security.rs      # 安全验证和工具执行
│   ├── tests.rs         # 综合测试套件
│   └── demo.rs          # 演示版本
├── .claude/             # 配置目录（自动创建）
│   ├── settings.json    # 用户配置
│   └── settings.local.json # 本地配置
├── Cargo.toml           # 项目配置
├── PRD.md               # 产品需求文档
└── README.md            # 本文件
```

### 构建和测试

```bash
# 检查代码
cargo check

# 构建项目
cargo build --release

# 运行测试
cargo test

# 运行 clippy
cargo clippy -- -D warnings

# 格式化代码
cargo fmt
```

### 技术栈

- **语言**: Rust 2021 Edition
- **异步运行时**: Tokio
- **HTTP 客户端**: Reqwest
- **CLI 框架**: Clap
- **序列化**: Serde + Serde JSON
- **用户交互**: Dialoguer + Console
- **错误处理**: Thiserror + Backoff
- **日志记录**: Tracing + Tracing-subscriber
- **安全验证**: 自定义验证模块
- **测试框架**: 内置测试 + Mockito (模拟 API)

## 📝 示例

### 示例 1：文件分析

```bash
cargo run --release -- --prompt "分析 src/main.rs 文件的内容"
```

### 示例 2：代码生成

```bash
cargo run --release -- --prompt "帮我写一个 Rust 的二叉树实现"
```

### 示例 3：文件操作

在交互模式中：
```
You: 读取 Cargo.toml 文件
Claude: [显示文件内容]

You: 在项目根目录创建一个 test.txt 文件，内容为 "Hello World"
Claude: [执行文件写入]

You: 列出所有 .rs 文件
Claude: [使用 glob 模式搜索并显示结果]
```

## 🔒 安全

- API 密钥通过多种方式安全存储（环境变量、配置文件）
- 文件操作遵循系统权限
- 命令执行有适当的错误处理
- 输入验证防止注入攻击

## 📊 性能

- 启动时间: < 1 秒
- API 响应: < 5 秒（正常网络条件）
- 内存使用: < 50MB
- 低 CPU 占用

## 🗺️ 路线图

### Phase 1 ✅ (已完成)
- ✅ 基础 CLI 框架
- ✅ Claude API 集成
- ✅ 核心工具实现
- ✅ 配置管理系统

### Phase 2 🔄 (进行中)
- 🔄 完善测试覆盖
- 🔄 错误处理优化
- 🔄 性能优化
- 🔄 插件系统架构

### Phase 3 📋 (未来)
- 📋 Rust LSP 集成
- 📋 代理服务器支持
- 📋 Web 界面
- 📋 云端同步

## 🤝 贡献

欢迎贡献！请随时提交 Pull Request 或创建 Issue。

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [Anthropic](https://www.anthropic.com/) - Claude API
- [Rust Community](https://www.rust-lang.org/) - 优秀的工具和库

## 📮 联系方式

- GitHub Issues: [提交问题](https://github.com/yourusername/rust-claude-code/issues)
- Email: your.email@example.com

---

**注意**: 本项目需要有效的 Anthropic API 密钥才能使用。请访问 [Anthropic官网](https://www.anthropic.com/) 获取 API 密钥。
