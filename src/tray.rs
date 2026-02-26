//! 系统托盘支持：提供托盘图标、菜单和交互功能。

use anyhow::{anyhow, Result};
use clipboard_rs::Clipboard;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tray_item::{IconSource, TrayItem};

/// 托盘回调事件类型。
#[derive(Debug, Clone, PartialEq)]
pub enum TrayEvent {
    /// 退出程序
    Quit,
    /// 打开配置文件
    OpenConfig,
}

/// 系统托盘管理器。
pub struct TrayManager {
    tray: TrayItem,
    event_rx: mpsc::Receiver<TrayEvent>,
    shutdown: Arc<AtomicBool>,
}

impl TrayManager {
    /// 创建新的托盘管理器。
    ///
    /// # 参数
    ///
    /// * `config_path` - 配置文件的路径，用于"打开配置文件"菜单项
    pub fn new(config_path: std::path::PathBuf) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));

        // 创建托盘项，使用系统默认图标
        let icon_source = IconSource::Resource("");
        let mut tray = TrayItem::new("LAN Clipboard Sync", icon_source)
            .map_err(|e| anyhow!("failed to create tray item: {}", e))?;

        // 添加菜单项
        let config_path_clone = config_path.clone();
        let event_tx_clone = event_tx.clone();
        tray.add_menu_item("Copy Config Path", move || {
            tracing::info!("Copy Config Path menu item clicked");
            copy_config_dir_to_clipboard(&config_path_clone);
            let _ = event_tx_clone.send(TrayEvent::OpenConfig);
        })
        .map_err(|e| anyhow!("failed to add Copy Config Path menu item: {}", e))?;

        let shutdown_clone = Arc::clone(&shutdown);
        let event_tx_clone = event_tx.clone();
        tray.add_menu_item("Quit", move || {
            tracing::info!("Quit menu item clicked");
            shutdown_clone.store(true, Ordering::SeqCst);
            let _ = event_tx_clone.send(TrayEvent::Quit);
        })
        .map_err(|e| anyhow!("failed to add Quit menu item: {}", e))?;

        tracing::info!("system tray initialized");

        Ok(Self { tray, event_rx, shutdown })
    }

    /// 尝试从托盘接收事件（非阻塞）。
    ///
    /// 返回 `Some(event)` 如果有事件可用，`None` 如果没有事件。
    pub fn try_recv(&self) -> Option<TrayEvent> {
        self.event_rx.try_recv().ok()
    }

    /// 阻塞等待托盘事件。
    pub fn recv(&self) -> Result<TrayEvent> {
        self.event_rx.recv().map_err(|e| anyhow!("failed to receive tray event: {}", e))
    }

    /// 检查是否应该关闭程序。
    pub fn should_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }
}

/// 将配置文件所在目录的路径复制到剪贴板。
fn copy_config_dir_to_clipboard(config_path: &std::path::Path) {
    tracing::debug!("config file path: {}", config_path.display());

    // 获取配置文件所在的目录路径
    if let Some(dir_path) = config_path.parent() {
        let dir_str = dir_path.to_string_lossy().to_string();
        tracing::info!("copying config directory to clipboard: {}", dir_str);

        // 使用 clipboard_rs::ClipboardContext 将路径复制到剪贴板
        match clipboard_rs::ClipboardContext::new() {
            Ok(clipboard) => {
                if let Err(e) = clipboard.set_text(dir_str.clone()) {
                    tracing::error!("failed to copy to clipboard: {}", e);
                } else {
                    tracing::info!("successfully copied to clipboard: {}", dir_str);
                }
            }
            Err(e) => {
                tracing::error!("failed to create clipboard context: {}", e);
            }
        }
    } else {
        tracing::error!("config file has no parent directory");
    }
}