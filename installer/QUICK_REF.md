# 安装程序开发快速参考

## 项目结构

```
lan-clipboard-sync/
├── build.sh                          # 统一构建入口脚本
├── BUILD.md                          # 详细构建文档
├── build/                            # 构建脚本目录
│   ├── windows.sh                    # Linux 上运行 Windows 构建
│   ├── windows.bat                   # Windows 上运行 Windows 构建
│   └── linux.sh                      # Linux DEB 包构建脚本
├── installer/                        # 安装程序源文件
│   ├── windows/
│   │   ├── lan-clipboard-sync.iss    # Inno Setup 脚本
│   │   └── config-template.toml      # Windows 配置模板
│   ├── linux/
│   │   ├── debian/                   # DEB 包控制文件
│   │   │   ├── control
│   │   │   ├── changelog
│   │   │   ├── copyright
│   │   │   └── postinst              # 安装后脚本（可执行）
│   │   └── config-template.toml      # Linux 配置模板
│   └── README.sh                     # 目录结构说明
└── dist/                             # 构建输出（不提交到 git）
    ├── windows/
    │   └── lan-clipboard-sync-0.1.0-setup.exe
    └── linux/
        └── lan-clipboard-sync-0.1.0.deb
```

## 快速命令

### Linux 上构建 Linux DEB 包
```bash
./build.sh linux
```

### Windows 上构建 Windows 安装程序
```cmd
build.bat
```

### Linux 上构建所有平台
```bash
./build.sh all
```

## 文件说明

### 构建脚本
- `build.sh` - 统一入口，根据参数选择构建目标
- `build/linux.sh` - Linux DEB 包构建，包含依赖检查和打包
- `build/windows.sh` - Linux 上交叉编译 Windows 程序
- `build/windows.bat` - Windows 本地构建脚本

### 安装程序文件
- `installer/windows/*.iss` - Inno Setup 安装脚本，定义安装流程
- `installer/linux/debian/*` - DEB 包元数据和控制脚本

### 配置模板
- `config-template.toml` - 默认配置，安装时复制到系统

## 关键配置位置

| 平台 | 配置文件 | 安装位置 |
|------|---------|---------|
| Windows | config-template.toml | %APPDATA%\lan-clipboard-sync\ |
| Linux | config-template.toml | /etc/lan-clipboard-sync/ |

## 版本更新

需要更新版本号时，修改以下文件：
1. `Cargo.toml` - `version` 字段
2. `installer/windows/lan-clipboard-sync.iss` - `AppVersion` 定义
3. `build/windows.sh` - `VERSION` 变量
4. `build/linux.sh` - `VERSION` 变量

## 测试清单

- [ ] Linux DEB 包安装
- [ ] Linux 服务启动和停止
- [ ] Windows 安装程序安装
- [ ] Windows 程序运行
- [ ] 配置文件正确生成
- [ ] 跨平台通信测试
- [ ] 文件下载功能
- [ ] 卸载功能

## 常见问题

详见 BUILD.md 中的"故障排除"章节。