#!/bin/bash
# Linux 构建脚本

set -e

VERSION="0.1.0"
PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
INSTALLER_DIR="$PROJECT_ROOT/installer/linux"
DEBIAN_DIR="$INSTALLER_DIR/debian"
OUTPUT_DIR="$PROJECT_ROOT/dist/linux"
TEMP_DIR="/tmp/lan-clipboard-sync-build"

echo "=========================================="
echo "LAN Clipboard Sync - Linux 构建脚本"
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

# 检查必要的构建工具
echo "检查构建工具..."
MISSING_DEPS=()

for cmd in dpkg-deb fakeroot; do
    if ! command -v $cmd &> /dev/null; then
        MISSING_DEPS+=($cmd)
    fi
done

if [ ${#MISSING_DEPS[@]} -gt 0 ]; then
    echo "错误: 缺少必要的构建工具: ${MISSING_DEPS[*]}"
    echo "请运行: sudo apt install ${MISSING_DEPS[*]}"
    exit 1
fi

echo "构建工具检查完成"
echo ""

# 检查系统依赖
echo "检查系统依赖..."
MISSING_LIBS=()

# 检查 GTK3
if ! pkg-config --exists gtk+-3.0; then
    MISSING_LIBS+=("libgtk-3-dev")
fi

# 检查 appindicator
if ! pkg-config --exists appindicator3-0.1 && ! pkg-config --exists ayatana-appindicator3-0.1; then
    MISSING_LIBS+=("libappindicator3-dev")
fi

if [ ${#MISSING_LIBS[@]} -gt 0 ]; then
    echo "警告: 缺少系统库: ${MISSING_LIBS[*]}"
    echo "请运行: sudo apt install ${MISSING_LIBS[*]}"
    echo ""
    read -p "是否继续构建？（托盘图标功能可能不可用）(y/n) " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi
echo ""

# 创建输出目录
echo "创建输出目录..."
mkdir -p "$OUTPUT_DIR"
echo ""

# 构建 Linux 可执行文件
echo "=========================================="
echo "开始构建 Linux 可执行文件..."
echo "=========================================="
cd "$PROJECT_ROOT"

cargo build --release

echo "构建完成！"
echo ""

# 准备 DEB 包结构
echo "=========================================="
echo "准备 DEB 包结构..."
echo "=========================================="

# 清理临时目录
rm -rf "$TEMP_DIR"
mkdir -p "$TEMP_DIR"

# 创建 DEB 包目录结构
DEB_ROOT="$TEMP_DIR/lan-clipboard-sync-$VERSION"
mkdir -p "$DEB_ROOT"
mkdir -p "$DEB_ROOT/DEBIAN"
mkdir -p "$DEB_ROOT/usr/bin"
mkdir -p "$DEB_ROOT/etc/lan-clipboard-sync"
mkdir -p "$DEB_ROOT/usr/share/lan-clipboard-sync"
mkdir -p "$DEB_ROOT/usr/share/doc/lan-clipboard-sync"
mkdir -p "$DEB_ROOT/var/lib/lan-clipboard-sync"
mkdir -p "$DEB_ROOT/lib/systemd/system"

# 复制可执行文件
echo "复制可执行文件..."
cp target/release/lan-clipboard-sync "$DEB_ROOT/usr/bin/"
chmod +x "$DEB_ROOT/usr/bin/lan-clipboard-sync"

# 复制配置文件模板
echo "复制配置文件模板..."
cp "$INSTALLER_DIR/config-template.toml" "$DEB_ROOT/usr/share/lan-clipboard-sync/"

# 复制文档
echo "复制文档..."
cp README.md "$DEB_ROOT/usr/share/doc/lan-clipboard-sync/"
cp LICENSE "$DEB_ROOT/usr/share/doc/lan-clipboard-sync/"

# 复制 DEBIAN 控制文件
echo "复制 DEBIAN 控制文件..."
cp "$DEBIAN_DIR/control" "$DEB_ROOT/DEBIAN/"
cp "$DEBIAN_DIR/changelog" "$DEB_ROOT/DEBIAN/"
cp "$DEBIAN_DIR/copyright" "$DEB_ROOT/DEBIAN/"
cp "$DEBIAN_DIR/postinst" "$DEB_ROOT/DEBIAN/"

# 设置 postinst 为可执行
chmod 755 "$DEB_ROOT/DEBIAN/postinst"

# 创建 conffiles 标记配置文件
echo "标记配置文件..."
echo "/etc/lan-clipboard-sync/config.toml" > "$DEB_ROOT/DEBIAN/conffiles"

# 计算 installed-size
echo "计算包大小..."
INSTALLED_SIZE=$(du -sk "$DEB_ROOT" | cut -f1)
echo "Installed-Size: $INSTALLED_SIZE" >> "$DEB_ROOT/DEBIAN/control"

echo ""
echo "DEB 包结构准备完成"
echo ""

# 构建 DEB 包
echo "=========================================="
echo "构建 DEB 包..."
echo "=========================================="
cd "$TEMP_DIR"

fakeroot dpkg-deb --build "lan-clipboard-sync-$VERSION"

echo ""
echo "DEB 包构建完成！"
echo ""

# 移动到输出目录
echo "移动到输出目录..."
mv "lan-clipboard-sync-${VERSION}.deb" "$OUTPUT_DIR/"

# 清理临时目录
echo "清理临时目录..."
rm -rf "$TEMP_DIR"

echo ""
echo "=========================================="
echo "Linux 构建完成！"
echo "=========================================="
echo ""
echo "生成的文件:"
echo "✓ $OUTPUT_DIR/lan-clipboard-sync-${VERSION}.deb"
echo ""
echo "包信息:"
dpkg-deb -I "$OUTPUT_DIR/lan-clipboard-sync-${VERSION}.deb"
echo ""
echo "安装方法:"
echo "sudo dpkg -i $OUTPUT_DIR/lan-clipboard-sync-${VERSION}.deb"
echo ""
echo "安装后配置:"
echo "配置文件: /etc/lan-clipboard-sync/config.toml"
echo "启用服务: sudo systemctl enable lan-clipboard-sync"
echo "启动服务: sudo systemctl start lan-clipboard-sync"
echo ""