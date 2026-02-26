#!/bin/bash
# Windows 构建脚本

set -e

VERSION="0.1.0"
PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
INSTALLER_DIR="$PROJECT_ROOT/installer/windows"
OUTPUT_DIR="$PROJECT_ROOT/dist/windows"

echo "=========================================="
echo "LAN Clipboard Sync - Windows 构建脚本"
echo "版本: $VERSION"
echo "=========================================="
echo ""

# 检查 Rust 工具链
echo "检查 Rust 工具链..."
if ! command -v cargo &> /dev/null; then
    echo "错误: 未找到 Rust 工具链，请先安装 Rust"
    echo "访问: https://www.rust-lang.org/tools/install"
    exit 1
fi

echo "Rust 工具链检查完成"
echo ""

# 检查 Inno Setup 编译器
echo "检查 Inno Setup 编译器..."
if ! command -v ISCC.exe &> /dev/null; then
    echo "警告: 未找到 Inno Setup 编译器 (ISCC.exe)"
    echo "请从以下地址下载并安装: https://jrsoftware.org/isdl.php"
    echo ""
    read -p "是否继续构建可执行文件（不创建安装程序）？(y/n) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
    CREATE_INSTALLER=false
else
    CREATE_INSTALLER=true
fi
echo ""

# 创建输出目录
echo "创建输出目录..."
mkdir -p "$OUTPUT_DIR"
echo ""

# 构建 Windows 可执行文件
echo "=========================================="
echo "开始构建 Windows 可执行文件..."
echo "=========================================="
cd "$PROJECT_ROOT"

# 设置目标架构为 x86_64-pc-windows-msvc
echo "设置目标架构: x86_64-pc-windows-msvc"

# 检查是否已安装 Windows 目标
if ! rustup target list --installed | grep -q "x86_64-pc-windows-msvc"; then
    echo "安装 Windows 目标..."
    rustup target add x86_64-pc-windows-msvc
fi

# 使用交叉编译或本地编译
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    # Windows 本地编译
    echo "使用本地 Windows 环境编译..."
    cargo build --release
else
    # Linux 上交叉编译
    echo "使用交叉编译（需要安装 x86_64-pc-windows-msvc 工具链）..."
    cargo build --release --target x86_64-pc-windows-msvc
fi

echo "构建完成！"
echo ""

# 复制可执行文件到安装程序目录
echo "复制文件到安装程序目录..."
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    cp target/release/lan-clipboard-sync.exe "$INSTALLER_DIR/"
else
    cp target/x86_64-pc-windows-msvc/release/lan-clipboard-sync.exe "$INSTALLER_DIR/"
fi
echo ""

# 创建安装程序
if [ "$CREATE_INSTALLER" = true ]; then
    echo "=========================================="
    echo "创建安装程序..."
    echo "=========================================="
    
    cd "$INSTALLER_DIR"
    
    # 运行 Inno Setup 编译器
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        ISCC.exe lan-clipboard-sync.iss
    else
        # 在 Linux 上可能需要通过 Wine 运行
        echo "注意: 在 Linux 上编译需要通过 Wine 运行 ISCC.exe"
        wine ISCC.exe lan-clipboard-sync.iss
    fi
    
    echo ""
    echo "安装程序创建完成！"
    echo ""
else
    echo "跳过安装程序创建"
    echo ""
    echo "可执行文件位置: $INSTALLER_DIR/lan-clipboard-sync.exe"
    echo ""
fi

# 查找生成的安装程序
if [ "$CREATE_INSTALLER" = true ]; then
    echo "=========================================="
    echo "生成的文件:"
    echo "=========================================="
    
    if [ -f "$INSTALLER_DIR/lan-clipboard-sync-${VERSION}-setup.exe" ]; then
        mv "$INSTALLER_DIR/lan-clipboard-sync-${VERSION}-setup.exe" "$OUTPUT_DIR/"
        echo "✓ $OUTPUT_DIR/lan-clipboard-sync-${VERSION}-setup.exe"
    fi
    
    for f in "$INSTALLER_DIR"/*.exe; do
        if [ -f "$f" ]; then
            size=$(du -h "$f" | cut -f1)
            echo "  $(basename $f) ($size)"
        fi
    done
    echo ""
fi

echo "=========================================="
echo "Windows 构建完成！"
echo "=========================================="
echo ""
echo "输出目录: $OUTPUT_DIR"
echo ""