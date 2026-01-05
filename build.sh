#!/bin/bash

# 构建脚本 - 用于绕过 cargo 的系统问题

echo "🦀 Rust Claude Code 构建脚本"
echo "================================"
echo ""

# 检查 ANTHROPIC_API_KEY
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo "⚠️  警告: ANTHROPIC_API_KEY 未设置"
    echo "   请先设置: export ANTHROPIC_API_KEY=your_key"
    echo ""
fi

# 尝试使用 cargo
echo "正在尝试构建..."
if cargo build --release 2>&1; then
    echo ""
    echo "✅ 构建成功!"
    echo ""
    echo "运行程序:"
    echo "  ./target/release/rust-claude-code --help"
    echo ""
    echo "示例:"
    echo "  ./target/release/rust-claude-code --prompt '列出当前目录的 Rust 文件'"
    echo "  ./target/release/rust-claude-code  # 交互模式"
else
    echo ""
    echo "❌ 构建失败"
    echo ""
    echo "这可能是因为当前环境中的系统配置问题。"
    echo "请在正常的终端环境中运行此脚本。"
    exit 1
fi
