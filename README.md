# 🦀 Rust Claude Code

[![Rust](https://img.shields.io/badge/rust-2021%20Edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/yourusername/rust-claude-code)

A high-performance, secure Rust implementation of Claude AI code assistant CLI tool, providing intelligent code assistance through Anthropic Claude API.

## 📁 Documentation

- **English**: This README.md
- **中文文档**: [README_CN.md](README_CN.md)
- **PRD**: [PRD.md](PRD.md) (Product Requirements Document)

## ✨ Features

- 🚀 **High Performance** - Built with Rust for fast response and low memory usage
- 🔒 **Security-First** - Type safety, memory safety, and comprehensive input validation
- 🛠️ **Tool Support** - File operations, command execution, and more
- ⚙️ **Configuration Management** - Flexible config system supporting multiple sources
- 🎨 **Rich UI** - Beautiful colored terminal output
- 🔄 **Interactive Mode** - Support for both interactive conversations and single prompts
- 🌐 **Cross-Platform** - Works on Windows, macOS, and Linux

## 🚀 Quick Start

### Requirements

- Rust 2021 Edition or later
- Anthropic API key

### Installation

1. Clone repository
```bash
git clone https://github.com/yourusername/rust-claude-code.git
cd rust-claude-code
```

2. Build project
```bash
./build.sh build
# or manually:
cargo build --release
```

3. Configure API key

**Method 1: Environment Variables**
```bash
export ANTHROPIC_API_KEY=your_api_key_here
export ANTHROPIC_BASE_URL=https://api.anthropic.com  # or custom proxy
```

**Method 2: Configuration File**
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

### Usage

#### Interactive Mode

```bash
# Use API key from environment
./target/release/rust-claude-code

# Specify API key via command line
./target/release/rust-claude-code --api-key your_api_key

# Set maximum conversation turns
./target/release/rust-claude-code --max-turns 20
```

#### Single Prompt Mode

```bash
# Execute single prompt
./target/release/rust-claude-code --prompt "Write a Hello World program"

# With API key
./target/release/rust-claude-code --api-key your_key --prompt "Explain this code"
```

## ⚙️ Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ANTHROPIC_API_KEY` | Anthropic API key | - |
| `ANTHROPIC_AUTH_TOKEN` | API authentication token (same as above) | - |
| `ANTHROPIC_BASE_URL` | API base URL | `https://api.anthropic.com` |
| `API_TIMEOUT_MS` | API timeout in milliseconds | `120000` |

### Command Line Options

```
rust-claude-code [OPTIONS]

Options:
  -k, --api-key <API_KEY>           Anthropic API key (overrides config)
  -m, --max-turns <MAX_TURNS>      Maximum conversation turns [default: 10]
  -p, --prompt <PROMPT>            Single prompt mode
  -u, --api-url <API_URL>          API base URL (overrides config)
  -t, --timeout <TIMEOUT>          Timeout in seconds
      --show-config                Show configuration file path
  -h, --help                       Show help information
  -V, --version                    Show version information
```

## 🛠️ Development

### Project Structure

```
rust-claude-code/
├── src/
│   ├── main.rs          # Main program entry point
│   ├── config.rs        # Configuration management module
│   ├── error.rs         # Error handling and retry mechanisms
│   ├── security.rs      # Security validation and tool execution
│   ├── performance.rs   # Performance optimizations for file handling
│   ├── tests.rs         # Comprehensive test suite
│   └── demo.rs          # Demo version
├── .claude/             # Configuration directory (auto-created)
├── Cargo.toml           # Project configuration
├── build.sh             # Build and deployment script
├── README.md            # English documentation (this file)
├── README_CN.md         # Chinese documentation
├── PRD.md               # Product Requirements Document
└── LICENSE           # MIT License
```

### Build and Test

```bash
# Check code
cargo check

# Build project
./build.sh build

# Run tests
./build.sh test

# Run linter
./build.sh check

# Format code
cargo fmt
```

## 📊 Performance

- **Startup time**: < 1 second
- **API response**: < 5 seconds (normal network conditions)
- **Memory usage**: < 50MB
- **Low CPU usage**: Efficient async operations

## 🗺️ Roadmap

### Phase 1 ✅ (Complete)
- ✅ Basic CLI framework
- ✅ Claude API integration
- ✅ Core tool implementation
- ✅ Configuration management system
- ✅ Security enhancements
- ✅ Error handling and retry mechanisms

### Phase 2 🔄 (In Progress)
- 🔄 Rust LSP integration
- 🔄 Plugin system architecture
- 🔄 Web interface development

## 🧪 Testing

The project includes comprehensive testing:

```bash
# Run all tests
cargo test
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit Pull Requests or create Issues.

1. Fork this repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Anthropic](https://www.anthropic.com/) - Claude API
- [Rust Community](https://www.rust-lang.org/) - Excellent tools and libraries

## 📮 Contact

- GitHub Issues: [Submit issues](https://github.com/yourusername/rust-claude-code/issues)
- Email: your.email@example.com

---

**Note**: This project requires a valid Anthropic API key to function. Please visit [Anthropic's website](https://www.anthropic.com/) to obtain an API key.