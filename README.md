# lan-clipboard-sync

一个使用 Rust 编写的局域网剪贴板同步工具，支持 **Windows** 与 **Linux**（KDE/X11/Wayland）。  
本项目无可视化主界面，通过系统托盘图标常驻后台运行，在多台设备之间自动同步剪贴板中的 **文本、图片与文件**。

## 功能特性

- **跨平台**：支持 Windows 与 Linux（X11 / Wayland，根据环境变量自动选择剪贴板后端）。
- **多类型剪贴板同步**：
  - 文本（UTF-8）
  - 图片（通过 PNG/JPEG 编码）
  - 文件（路径与内容，支持多文件）
- **加密传输**：使用 ChaCha20-Poly1305 对称加密保护局域网内的剪贴板数据。
- **可配置**：通过 JSON 或 TOML 文件配置端口、对端设备、密钥与最大文件大小。
- **图形化配置**：提供基于 egui 的配置 UI 窗口，可从托盘菜单打开，支持可视化编辑并保存配置。
- **自动化测试**：包含单元测试与端到端集成测试。

## 安装与构建

### 本地构建

```bash
git clone <your-repo-url>
cd lan-clipboard-sync
cargo build --release
```

### 跨平台构建（build.sh）

项目提供 `build.sh` 脚本，可同时构建 Windows 与 Linux x86_64 版本：

```bash
./build.sh
```

构建产物输出到 `build/` 目录：

- `build/lan-clipboard-sync-windows-x86_64.exe`
- `build/lan-clipboard-sync-linux-x86_64`

构建 Windows 版本需要配置 `x86_64-pc-windows-gnu` 工具链。

## 运行时依赖

- **Rust 工具链**：1.75+（建议最新版 stable）
- **Linux 额外依赖**（托盘图标与 Wayland 剪贴板）：
  - `gtk3`
  - `libappindicator-gtk3` 或 `libayatana-appindicator3`
  - Wayland 下需 `wl-clipboard`
  - 例如在 Arch / Manjaro：
    ```bash
    sudo pacman -S gtk3 libappindicator-gtk3 wl-clipboard
    ```
  - 在 Debian / Ubuntu：
    ```bash
    sudo apt install libgtk-3-dev libappindicator3-dev wl-clipboard
    ```

## 配置文件

默认配置路径：

- **Linux**：`$XDG_CONFIG_HOME/lan-clipboard-sync/config.toml` 或 `~/.config/lan-clipboard-sync/config.toml`
- **Windows**：`%APPDATA%\lan-clipboard-sync\config.toml`

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
   或直接运行已构建的可执行文件：
   ```bash
   ./lan-clipboard-sync   # Linux
   lan-clipboard-sync.exe # Windows
   ```
4. 指定自定义配置文件路径：
   ```bash
   lan-clipboard-sync -c /path/to/your/config.toml
   ```
   或使用长选项：
   ```bash
   lan-clipboard-sync --config /path/to/your/config.toml
   ```
5. 程序启动后会在系统托盘出现一个图标，右键菜单提供：
   - **配置**：打开图形化配置窗口，可视化编辑并保存配置（需重启后生效）
   - **复制配置路径**：将配置文件所在目录路径复制到剪贴板，便于在文件管理器中定位
   - **Quit**：退出程序

## 运行机制概览

- 程序在本机监听配置中的 `listen_port`，使用 TCP 接收来自其他设备的剪贴板更新。
- 程序监控本机剪贴板，一旦内容变化（文本/图片/文件）且未超出配置的最大文件大小，即对内容进行加密并广播到所有 `peers`。
- 收到来自其他设备的更新后，程序会在本机应用到剪贴板，同时避免引发无限循环广播（去重与防回声）。
- **文件同步**：接收到的文件会保存到用户下载目录下的 `lan-clipboard` 子目录，并按时间戳创建子文件夹（格式：`YYYYMMDD-HHMMSS`），便于区分不同批次的同步文件。
  - Linux：`~/Downloads/lan-clipboard/`
  - Windows：`%USERPROFILE%\Downloads\lan-clipboard\`

## 日志

默认日志级别为 `info`。可通过环境变量 `RUST_LOG` 调整：

```bash
RUST_LOG=debug lan-clipboard-sync
RUST_LOG=lan_clipboard_sync=trace lan-clipboard-sync
```

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

本项目在设计与实现过程中，多次借助 Cursor 编辑器中的 AI 编码助手进行需求梳理、架构讨论与部分代码生成。所有提交到仓库的改动均由作者本人进行审阅与测试，AI 仅作为辅助工具使用。
