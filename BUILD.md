# 构建与打包指南

本指南说明如何为 LAN Clipboard Sync 创建 Windows 和 Linux 的安装程序。

## 前置要求

### 通用要求
- Rust 工具链 1.75+
- Git

### Windows 要求
- Windows 10/11 x64
- Visual Studio Build Tools（或 Visual Studio）
- Inno Setup 6.x（用于创建安装程序）

### Linux 要求
- Linux x86_64（Ubuntu 20.04+, Debian 11+, Arch 等）
- GCC/Clang
- 构建工具：`dpkg-deb`, `fakeroot`

## 快速开始

### 构建 Linux DEB 包

```bash
# 仅构建 Linux
./build.sh linux

# 或直接运行 Linux 构建脚本
./build/linux.sh
```

生成的 DEB 包位于 `dist/linux/lan-clipboard-sync-0.1.0.deb`

### 构建 Windows 安装程序

在 Windows 系统上：

```bash
# 仅构建 Windows
build.bat windows

# 或直接运行 Windows 构建脚本
build\windows.bat
```

生成的安装程序位于 `dist/windows/lan-clipboard-sync-0.1.0-setup.exe`

### 构建所有平台

```bash
./build.sh all
```

## 详细说明

### Windows 安装程序

安装程序使用 [Inno Setup](https://jrsoftware.org/isinfo.php) 创建，包含以下特性：

- 自动安装到 `Program Files`
- 创建桌面快捷方式（可选）
- 开机自启动（可选）
- 自动创建配置文件
- 卸载时保留配置文件（可选）
- 中英文界面支持

#### 安装包内容
- `lan-clipboard-sync.exe` - 主程序
- `config-template.toml` - 配置文件模板
- `README.md` - 用户手册
- `LICENSE` - 许可证

#### 安装位置
- 程序文件：`C:\Program Files\LAN Clipboard Sync\`
- 配置文件：`%APPDATA%\lan-clipboard-sync\config.toml`

#### 使用方法

1. 双击运行 `lan-clipboard-sync-0.1.0-setup.exe`
2. 按照向导完成安装
3. 安装后首次运行会自动创建配置文件
4. 编辑配置文件添加对端设备信息

#### 服务管理

程序作为系统托盘应用运行，提供以下功能：
- 右键菜单可以重启、退出程序
- 右键菜单可以打开配置文件
- 系统托盘图标显示运行状态

### Linux DEB 包

DEB 包使用标准的 Debian 打包工具创建，包含以下特性：

- systemd 服务集成
- 自动启动服务
- 专用系统用户 `lan-clipboard`
- 配置文件管理
- 自动依赖检查

#### 安装包内容
- `/usr/bin/lan-clipboard-sync` - 主程序
- `/etc/lan-clipboard-sync/config.toml` - 配置文件
- `/usr/share/lan-clipboard-sync/` - 共享资源
- `/var/lib/lan-clipboard-sync/` - 数据目录
- `/lib/systemd/system/lan-clipboard-sync.service` - 服务配置

#### 安装

```bash
sudo dpkg -i lan-clipboard-sync-0.1.0.deb
```

如果遇到依赖问题：

```bash
sudo apt install -f
```

#### 配置

编辑配置文件：

```bash
sudo nano /etc/lan-clipboard-sync/config.toml
```

修改以下配置：
- `listen_port` - 本机监听端口
- `secret_key` - 加密密钥（建议生成随机密钥）
- `peers` - 对端设备列表

生成随机密钥：

```bash
# Linux
openssl rand -hex 32

# 或使用
cat /dev/urandom | tr -dc 'a-f0-9' | fold -w 64 | head -n 1
```

#### 服务管理

```bash
# 启用服务（开机自启动）
sudo systemctl enable lan-clipboard-sync

# 启动服务
sudo systemctl start lan-clipboard-sync

# 停止服务
sudo systemctl stop lan-clipboard-sync

# 重启服务
sudo systemctl restart lan-clipboard-sync

# 查看状态
sudo systemctl status lan-clipboard-sync

# 查看日志
sudo journalctl -u lan-clipboard-sync -f
```

## 交叉编译

### 在 Linux 上构建 Windows 程序

需要安装 Windows 交叉编译工具链：

```bash
# 添加 Windows 目标
rustup target add x86_64-pc-windows-msvc

# 安装交叉编译工具
sudo apt install gcc-mingw-w64-x86-64
```

修改 `build/windows.sh` 中的编译命令。

### 在 Windows 上构建 Linux 程序

需要安装 Linux 交叉编译工具链（WSL）：

1. 安装 WSL2
2. 在 WSL 中运行构建脚本

## 自定义构建

### 修改版本号

更新以下文件中的版本号：
- `Cargo.toml` 中的 `version`
- `installer/windows/lan-clipboard-sync.iss` 中的 `AppVersion`
- `build/windows.sh` 中的 `VERSION`
- `build/linux.sh` 中的 `VERSION`

### 修改图标

Windows：
1. 准备 `.ico` 格式的图标文件
2. 放到 `installer/windows/` 目录
3. 修改 `.iss` 文件引用图标

Linux：
1. 准备 `.png` 格式的图标文件（多种尺寸）
2. 放到 `/usr/share/icons/hicolor/` 对应目录
3. 在 `debian/` 目录创建桌面文件

### 修改安装路径

Windows：
- 修改 `.iss` 文件中的 `DefaultDirName`

Linux：
- 修改 `build/linux.sh` 中的目录结构

## 故障排除

### Windows 构建问题

**问题**: 找不到 ISCC.exe
- 解决：下载并安装 Inno Setup：https://jrsoftware.org/isdl.php

**问题**: 链接错误
- 解决：安装 Visual Studio Build Tools 并配置正确的环境变量

### Linux 构建问题

**问题**: 缺少依赖库
- 解决：`sudo apt install libgtk-3-dev libappindicator3-dev`

**问题**: dpkg-deb 命令找不到
- 解决：`sudo apt install dpkg-dev fakeroot`

**问题**: 托盘图标不显示
- 解决：检查是否安装了正确的 appindicator 库
  ```bash
  sudo apt install libayatana-appindicator3-dev
  ```

### 运行时问题

**问题**: 服务无法启动
- 解决：查看日志：`sudo journalctl -u lan-clipboard-sync -n 50`

**问题**: 剪贴板同步不工作
- 解决：
  1. 检查端口是否被占用：`sudo netstat -tlnp | grep 5000`
  2. 检查防火墙设置
  3. 检查对端设备配置
  4. 查看日志排查错误

## 发布流程

1. 更新版本号
2. 运行测试：`cargo test --all`
3. 构建安装程序：`./build.sh all`
4. 在多台设备上测试安装程序
5. 创建 Git tag：`git tag -a v0.1.0 -m "Release 0.1.0"`
6. 推送到 GitHub：`git push origin v0.1.0`
7. 在 GitHub Releases 上传安装程序

## 项目结构

```
lan-clipboard-sync/
├── build.sh                          # 统一构建脚本
├── build/
│   ├── windows.sh                    # Windows 构建脚本
│   └── linux.sh                      # Linux 构建脚本
├── installer/
│   ├── windows/
│   │   ├── lan-clipboard-sync.iss    # Inno Setup 安装脚本
│   │   └── config-template.toml      # Windows 配置模板
│   └── linux/
│       ├── debian/                   # DEB 包控制文件
│       │   ├── control
│       │   ├── changelog
│       │   ├── copyright
│       │   └── postinst
│       └── config-template.toml      # Linux 配置模板
└── dist/                             # 构建输出目录
    ├── windows/
    │   └── lan-clipboard-sync-0.1.0-setup.exe
    └── linux/
        └── lan-clipboard-sync-0.1.0.deb
```

## 许可证

MIT License - 详见 LICENSE 文件