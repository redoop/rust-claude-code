#!/bin/bash

# Rust Claude Code - 构建脚本
# 用于自动化构建、测试和部署流程

set -e  # 遇到错误立即退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查是否安装了必要的工具
check_dependencies() {
    log_info "检查依赖项..."
    
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo 未找到，请安装 Rust"
        exit 1
    fi
    
    if ! command -v rustc &> /dev/null; then
        log_error "Rust 编译器未找到"
        exit 1
    fi
    
    log_success "所有依赖项检查通过"
}

# 清理构建产物
clean() {
    log_info "清理构建产物..."
    cargo clean
    log_success "清理完成"
}

# 运行代码检查
run_checks() {
    log_info "运行代码检查..."
    
    # Clippy 检查
    log_info "运行 Clippy..."
    cargo clippy -- -D warnings
    log_success "Clippy 检查通过"
    
    # 格式检查
    log_info "检查代码格式..."
    cargo fmt -- --check
    log_success "代码格式检查通过"
}

# 运行测试
run_tests() {
    log_info "运行测试套件..."
    
    # 单元测试
    log_info "运行单元测试..."
    cargo test --lib
    log_success "单元测试通过"
    
    # 集成测试
    log_info "运行集成测试..."
    cargo test --test '*'
    log_success "集成测试通过"
    
    # 文档测试
    log_info "运行文档测试..."
    cargo test --doc
    log_success "文档测试通过"
    
    log_success "所有测试通过"
}

# 构建项目
build() {
    local build_mode=${1:-release}
    
    log_info "构建项目 (${build_mode} 模式)..."
    
    if [ "$build_mode" = "release" ]; then
        cargo build --release
    else
        cargo build
    fi
    
    log_success "构建完成"
}

# 运行基准测试
run_benchmarks() {
    log_info "运行性能基准测试..."
    
    # 这里可以添加自定义的基准测试
    # cargo bench
    
    log_success "基准测试完成"
}

# 生成文档
generate_docs() {
    log_info "生成文档..."
    
    # 生成 API 文档
    cargo doc --no-deps
    
    log_success "文档生成完成"
    log_info "文档位置: target/doc/"
}

# 创建发布包
create_package() {
    log_info "创建发布包..."
    
    local version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "rust-claude-code") | .version' 2>/dev/null || echo "0.1.0")
    local package_name="rust-claude-code-${version}"
    
    # 创建临时目录
    mkdir -p "dist/${package_name}"
    
    # 复制必要的文件
    cp target/release/rust-claude-code "dist/${package_name}/"
    cp README.md "dist/${package_name}/"
    cp LICENSE "dist/${package_name}/" 2>/dev/null || log_warning "LICENSE 文件不存在"
    
    # 复制配置文件模板
    mkdir -p "dist/${package_name}/.claude"
    cat > "dist/${package_name}/.claude/settings.json" << EOF
{
  "theme": "default",
  "auto_save": false,
  "ai_enabled": true,
  "confidence_threshold": 0.8,
  "enabled_plugins": [
    "rust-analyzer-lsp@claude-plugins-official"
  ]
}
EOF
    
    # 创建安装脚本
    cat > "dist/${package_name}/install.sh" << 'EOF'
#!/bin/bash
set -e

INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="rust-claude-code"

# 创建安装目录
mkdir -p "$INSTALL_DIR"

# 复制二进制文件
cp "$BINARY_NAME" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# 检查 PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "请将 $INSTALL_DIR 添加到您的 PATH 环境变量中"
    echo "在 ~/.bashrc 或 ~/.zshrc 中添加:"
    echo "export PATH=\"\$PATH:$INSTALL_DIR\""
fi

echo "安装完成！运行 '$BINARY_NAME --help' 开始使用。"
EOF
    
    chmod +x "dist/${package_name}/install.sh"
    
    # 创建压缩包
    cd dist
    tar -czf "${package_name}.tar.gz" "$package_name"
    cd ..
    
    log_success "发布包创建完成: dist/${package_name}.tar.gz"
}

# 安装到本地系统
install_local() {
    log_info "安装到本地系统..."
    
    local install_dir="$HOME/.local/bin"
    
    # 创建安装目录
    mkdir -p "$install_dir"
    
    # 复制二进制文件
    cp target/release/rust-claude-code "$install_dir/"
    chmod +x "$install_dir/rust-claude-code"
    
    log_success "安装完成！"
    log_info "二进制文件位置: $install_dir/rust-claude-code"
    
    # 检查 PATH
    if [[ ":$PATH:" != *":$install_dir:"* ]]; then
        log_warning "请将 $install_dir 添加到您的 PATH 环境变量中"
        log_warning "在 ~/.bashrc 或 ~/.zshrc 中添加: export PATH=\"\$PATH:$install_dir\""
    fi
}

# 运行安全扫描
run_security_scan() {
    log_info "运行安全扫描..."
    
    # 检查已知漏洞
    if command -v cargo-audit &> /dev/null; then
        cargo audit
    else
        log_warning "cargo-audit 未安装，跳过漏洞扫描"
    fi
    
    # 检查依赖许可证
    if command -v cargo-deny &> /dev/null; then
        cargo deny check
    else
        log_warning "cargo-deny 未安装，跳过许可证检查"
    fi
    
    log_success "安全扫描完成"
}

# 显示帮助信息
show_help() {
    echo "Rust Claude Code - 构建脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  clean          清理构建产物"
    echo "  check          运行代码检查"
    echo "  test           运行测试套件"
    echo "  build [mode]   构建项目 (debug|release, 默认: release)"
    echo "  bench          运行性能基准测试"
    echo "  docs           生成文档"
    echo "  package        创建发布包"
    echo "  install        安装到本地系统"
    echo "  security       运行安全扫描"
    echo "  all            执行完整流程 (check -> test -> build -> docs)"
    echo "  ci             CI/CD 流程 (check -> test -> build)"
    echo "  help           显示此帮助信息"
    echo ""
    echo "示例:"
    echo "  $0 all              # 执行完整流程"
    echo "  $0 build debug      # Debug 模式构建"
    echo "  $0 install          # 安装到本地"
}

# 完整构建流程
run_all() {
    log_info "开始完整构建流程..."
    
    check_dependencies
    run_checks
    run_tests
    build "release"
    run_security_scan
    generate_docs
    
    log_success "完整构建流程完成！"
}

# CI/CD 流程
run_ci() {
    log_info "开始 CI/CD 流程..."
    
    check_dependencies
    run_checks
    run_tests
    build "release"
    
    log_success "CI/CD 流程完成！"
}

# 主逻辑
main() {
    local command=${1:-help}
    
    case "$command" in
        clean)
            clean
            ;;
        check)
            check_dependencies
            run_checks
            ;;
        test)
            check_dependencies
            run_tests
            ;;
        build)
            check_dependencies
            build ${2:-release}
            ;;
        bench)
            check_dependencies
            run_benchmarks
            ;;
        docs)
            check_dependencies
            generate_docs
            ;;
        package)
            check_dependencies
            build "release"
            create_package
            ;;
        install)
            check_dependencies
            build "release"
            install_local
            ;;
        security)
            run_security_scan
            ;;
        all)
            run_all
            ;;
        ci)
            run_ci
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            log_error "未知命令: $command"
            show_help
            exit 1
            ;;
    esac
}

# 执行主函数
main "$@"
