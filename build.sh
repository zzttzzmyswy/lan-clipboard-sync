#!/bin/bash
# 统一构建脚本 - 支持跨平台构建

set -e

VERSION="0.1.0"
PROJECT_ROOT="$(cd "$(dirname "$0")" && pwd)"
OUTPUT_DIR="$PROJECT_ROOT/dist"

echo "=========================================="
echo "LAN Clipboard Sync - 统一构建脚本"
echo "版本: $VERSION"
echo "=========================================="
echo ""

# 解析命令行参数
BUILD_TARGET=${1:-all}

case "$BUILD_TARGET" in
    windows|linux|all)
        ;;
    *)
        echo "用法: $0 [windows|linux|all]"
        echo ""
        echo "选项:"
        echo "  windows  - 仅构建 Windows 安装程序"
        echo "  linux    - 仅构建 Linux DEB 包"
        echo "  all      - 构建所有平台（默认）"
        exit 1
        ;;
esac

echo "构建目标: $BUILD_TARGET"
echo ""

# 构建 Windows
if [ "$BUILD_TARGET" = "windows" ] || [ "$BUILD_TARGET" = "all" ]; then
    echo "=========================================="
    echo "构建 Windows 安装程序"
    echo "=========================================="
    echo ""
    
    if [ -x "$PROJECT_ROOT/build/windows.sh" ]; then
        bash "$PROJECT_ROOT/build/windows.sh"
        echo ""
    else
        echo "错误: 找不到 Windows 构建脚本"
        exit 1
    fi
fi

# 构建 Linux
if [ "$BUILD_TARGET" = "linux" ] || [ "$BUILD_TARGET" = "all" ]; then
    echo "=========================================="
    echo "构建 Linux DEB 包"
    echo "=========================================="
    echo ""
    
    if [ -x "$PROJECT_ROOT/build/linux.sh" ]; then
        bash "$PROJECT_ROOT/build/linux.sh"
        echo ""
    else
        echo "错误: 找不到 Linux 构建脚本"
        exit 1
    fi
fi

# 显示构建结果
echo "=========================================="
echo "构建完成！"
echo "=========================================="
echo ""
echo "生成的文件:"
echo ""

if [ "$BUILD_TARGET" = "windows" ] || [ "$BUILD_TARGET" = "all" ]; then
    echo "Windows:"
    find "$OUTPUT_DIR/windows" -name "*.exe" -o -name "lan-clipboard-sync.exe" 2>/dev/null | while read file; do
        if [ -f "$file" ]; then
            size=$(du -h "$file" | cut -f1)
            echo "  ✓ $file ($size)"
        fi
    done
    echo ""
fi

if [ "$BUILD_TARGET" = "linux" ] || [ "$BUILD_TARGET" = "all" ]; then
    echo "Linux:"
    find "$OUTPUT_DIR/linux" -name "*.deb" 2>/dev/null | while read file; do
        if [ -f "$file" ]; then
            size=$(du -h "$file" | cut -f1)
            echo "  ✓ $file ($size)"
        fi
    done
    echo ""
fi

echo "所有构建产物已保存到: $OUTPUT_DIR"
echo ""