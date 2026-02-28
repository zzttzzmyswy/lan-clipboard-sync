use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use lan_clipboard_sync::{AppConfig, CoreService};

#[cfg(any(target_os = "linux", target_os = "windows"))]
use lan_clipboard_sync::{TrayEvent, TrayManager};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 指定配置文件的路径
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// 仅启动配置 UI 窗口（供托盘菜单调用，内部使用）
    #[arg(long, hide = true)]
    config_ui: bool,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn run_with_tray(config: AppConfig, config_path: PathBuf) -> Result<()> {
    // 创建托盘管理器
    let tray = TrayManager::new(config_path.clone())?;
    tracing::info!("system tray initialized");

    // 创建并运行核心服务（独立线程，退出时随进程结束）
    let rt = tokio::runtime::Runtime::new()?;
    let mut core = CoreService::new(config)?;
    std::thread::spawn(move || {
        if let Err(e) = rt.block_on(core.run()) {
            tracing::error!("core service error: {e}");
        }
    });

    // 配置 UI 子进程句柄：进程内锁定，确保同时只打开一个配置窗口
    let mut config_ui_child: Option<std::process::Child> = None;

    // 在主线程中监听托盘事件
    loop {
        match tray.recv()? {
            TrayEvent::Quit => {
                tracing::info!("quit requested, exiting...");
                return Ok(());
            }
            TrayEvent::OpenConfigUI => {
                // 若已有子进程，检查是否已退出（try_wait 会回收僵尸进程）
                if let Some(mut child) = config_ui_child.take() {
                    // try_wait() -> Ok(None) 表示进程仍在运行
                    if let Ok(None) = child.try_wait() {
                        config_ui_child = Some(child);
                        tracing::debug!("配置窗口已打开，忽略重复点击");
                        continue;
                    }
                    // 已退出，config_ui_child 保持 None，可重新启动
                }

                let path = config_path.clone();
                if let Ok(exe) = std::env::current_exe() {
                    match std::process::Command::new(&exe)
                        .arg("--config-ui")
                        .args(["--config", path.to_string_lossy().as_ref()])
                        .spawn()
                    {
                        Ok(child) => config_ui_child = Some(child),
                        Err(e) => tracing::error!("无法启动配置窗口: {}", e),
                    }
                } else {
                    tracing::error!("无法获取当前可执行文件路径，无法打开配置窗口");
                }
            }
            TrayEvent::OpenConfig => {
                // 复制配置路径，无需额外处理
            }
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn run_without_tray(config: AppConfig) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let mut core = CoreService::new(config)?;
    rt.block_on(async move { core.run().await })
}

fn main() -> Result<()> {
    let args = Args::parse();

    init_logging();

    let config_path = resolve_config_path(args.config.clone());

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    if args.config_ui {
        // 仅运行配置 UI（子进程模式，解决关闭后无法再次打开的问题）
        lan_clipboard_sync::config_ui::run(config_path);
        return Ok(());
    }

    let config = AppConfig::load(config_path.clone())?;

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        run_with_tray(config, config_path)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        tracing::warn!("system tray not supported on this platform, running without tray");
        run_without_tray(config)
    }
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let ansi = supports_color();
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(ansi)
        .init();
}

/// 检测终端是否支持色彩，避免在不支持 ANSI 的终端中输出转义序列
fn supports_color() -> bool {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        return false;
    }
    std::env::var("TERM").map_or(false, |term| term != "dumb")
}

fn resolve_config_path(arg: Option<PathBuf>) -> PathBuf {
    // 如果通过命令行参数指定了路径，则使用该路径；否则使用默认路径
    arg.unwrap_or_else(AppConfig::default_path)
}