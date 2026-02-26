# lan-clipboard-sync

一个使用 Rust 编写的局域网剪贴板同步工具，支持 **Windows** 与 **Linux KDE 桌面**。  
本项目无可视化界面，通过系统托盘图标常驻后台运行，在多台设备之间自动同步剪贴板中的 **文本、图片与文件**。

## 功能特性

- **跨平台**：支持 Windows 与 Linux（KDE，X11/Wayland，依赖系统剪贴板与托盘实现）。
- **多类型剪贴板同步**：
  - 文本（UTF-8）
  - 图片（通过 PNG/JPEG 编码）
  - 文件（路径与内容）
- **加密传输**：使用对称加密（`chacha20poly1305`）保护局域网内的剪贴板数据。
- **可配置**：通过 JSON 或 TOML 文件配置端口、对端设备、密钥与最大文件大小。
- **自动化测试**：包含单元测试与端到端集成测试。

## 安装与构建

```bash
git clone <your-repo-url>
cd lan-clipboard-sync
cargo build --release
```

### 运行时依赖

- **Rust 工具链**：1.75+（建议最新版 stable）
- **Linux 额外依赖**（托盘图标）：
  - `gtk3`
  - `libappindicator-gtk3` 或 `libayatana-appindicator3`
  - 例如在 Arch / Manjaro：
    ```bash
    sudo pacman -S gtk3 libappindicator-gtk3
    ```
  - 在 Debian / Ubuntu：
    ```bash
    sudo apt install libgtk-3-dev libappindicator3-dev
    ```

## 配置文件

默认配置路径：

- **Linux**：`$XDG_CONFIG_HOME/lan-clipboard-sync/config.toml` 或 `~/.config/lan-clipboard-sync/config.toml`
- **Windows**：`%APPDATA%\\lan-clipboard-sync\\config.toml`

可以通过命令行参数 `-c` 或 `--config <path>` 覆盖默认路径。

### TOML 示例

```toml
# 本机监听的 TCP 端口
listen_port = 5000

# 对称加密密钥，32 字节的十六进制字符串（256-bit key）
secret_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

# 最大允许外发的文件大小（字节）
max_file_size = 10485760 # 10MB

[[peers]]
host = "192.168.1.23"
port = 5000

[[peers]]
host = "office-pc.local"
port = 5000
```

### JSON 示例

```json
{
  "listen_port": 5000,
  "secret_key": "af3c2f3b7c9eaf3c2f3b7c9eaf3c2f3b",
  "max_file_size": 10485760,
  "peers": [
    { "host": "192.168.1.23", "port": 5000 },
    { "host": "office-pc.local", "port": 5000 }
  ]
}
```

## 使用方式

1. 在每台需要同步的设备上安装并构建本程序。
2. 在每台设备上创建配置文件，填写相同的 `secret_key`，并在 `peers` 中列出其他设备的 IP/域名与端口。
3. 启动程序：
   ```bash
   cd lan-clipboard-sync
   cargo run --release
   ```
4. 指定自定义配置文件路径：
   ```bash
   cd lan-clipboard-sync
   cargo run --release --config /path/to/your/config.toml
   ```
   或使用短选项：
   ```bash
   cargo run --release -c /path/to/your/config.toml
   ```
5. 程序启动后会在系统托盘出现一个图标，可通过右键菜单进行：
   - **Restart**：重启后台服务进程
   - **Quit**：退出程序
   - **Open Config**：在默认编辑器中打开配置文件

## 运行机制概览

- 程序在本机监听配置中的 `listen_port`，使用 TCP 接收来自其他设备的剪贴板更新。
- 程序监控本机剪贴板，一旦内容变化（文本/图片/文件）且未超出配置的最大文件大小，即对内容进行加密并广播到所有 `peers`。
- 收到来自其他设备的更新后，程序会在本机应用到剪贴板，同时避免引发无限循环广播。

## 安全说明

- 配置文件中的 `secret_key` 是所有节点共享的对称密钥，请妥善保管，避免泄露。
- 建议：
  - 使用 32 字节（64 位十六进制字符串）的随机密钥。
  - 限制配置文件的读写权限，仅允许当前用户访问。
  - 在受信任的局域网环境中使用本工具。

## 测试

运行所有测试（单元测试 + 集成测试）：

```bash
cargo test --all
```

测试内容包括：

- 配置解析与校验。
- 加密/解密正确性与协议编码/解码。
- 剪贴板抽象层逻辑（通过 mock 测试，不依赖真实系统剪贴板）。
- 简单端到端网络传输与剪贴板同步流程（使用本地端口）。

## 开源协议

本项目以 **MIT License** 开源，详情见项目根目录中的 `LICENSE` 文件。

## 致谢

本项目在设计与实现过程中，多次借助 Cursor 编辑器中的 AI 编码助手（基于 GPT-5.1）进行需求梳理、架构讨论与部分代码生成。所有提交到仓库的改动均由作者本人进行审阅与测试，AI 仅作为辅助工具使用。

