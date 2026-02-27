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
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn run_with_tray(config: AppConfig, config_path: PathBuf) -> Result<()> {
    // 创建托盘管理器
    let tray = TrayManager::new(config_path.clone())?;
    tracing::info!("system tray initialized");

    // 创建并运行核心服务
    let rt = tokio::runtime::Runtime::new()?;
    let mut core = CoreService::new(config)?;

    // 在独立线程中运行核心服务
    let (core_result_tx, _core_result_rx) = std::sync::mpsc::channel::<Result<()>>();
    let shutdown_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();

    std::thread::spawn(move || {
        let result = rt.block_on(async move { core.run().await });
        let _ = core_result_tx.send(result);
        shutdown_flag_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    // 在主线程中监听托盘事件
    loop {
        match tray.recv()? {
            TrayEvent::Quit => {
                tracing::info!("quit requested, exiting...");
                return Ok(());
            }
            TrayEvent::OpenConfig => {
                tracing::info!("config opened");
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