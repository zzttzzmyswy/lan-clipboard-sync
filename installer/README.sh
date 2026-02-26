#!/bin/bash
# 安装程序开发指南

set -e

echo "=========================================="
echo "LAN Clipboard Sync - 安装程序目录结构"
echo "=========================================="
echo ""

echo "installer/"
echo "├── windows/                      # Windows 安装程序"
echo "│   ├── lan-clipboard-sync.iss    # Inno Setup 安装脚本"
echo "│   └── config-template.toml      # Windows 配置模板"
echo "└── linux/                        # Linux 安装程序"
echo "    ├── debian/                   # DEB 包控制文件"
echo "    │   ├── control               # 包控制信息"
echo "    │   ├── changelog             # 变更日志"
echo "    │   ├── copyright             # 版权信息"
echo "    │   └── postinst              # 安装后脚本"
echo "    └── config-template.toml      # Linux 配置模板"
echo ""

echo "=========================================="
echo "快速开始"
echo "=========================================="
echo ""
echo "Linux 构建:"
echo "  ./build.sh linux"
echo ""
echo "Windows 构建（在 Windows 上）:"
echo "  build.bat"
echo ""
echo "构建所有平台:"
echo "  ./build.sh all"
echo ""

echo "=========================================="
echo "详细文档"
echo "=========================================="
echo ""
echo "请参阅 BUILD.md 获取完整的构建和打包指南"
echo ""