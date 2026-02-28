//! 系统托盘支持：提供托盘图标、菜单和交互功能。
//!
//! 图标在编译期通过 `include_bytes!` 内嵌到二进制中，运行时无需加载外部文件。

use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use tray_item::{IconSource, TrayItem};

/// 编译期内嵌的托盘图标（PNG）
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
const ICON_PNG: &[u8] = include_bytes!("../resources/icon.png");

/// 从内嵌的 PNG 创建托盘图标源。
///
/// - Linux (ksni)：解码 PNG 为像素数据，使用 IconSource::Data
/// - Windows：解码 PNG 并运行时生成 HICON，使用 IconSource::RawIcon
/// - macOS：解码 PNG 为像素数据，使用 IconSource::Data
fn embedded_tray_icon() -> Result<IconSource> {
    #[cfg(target_os = "linux")]
    {
        use image::ImageReader;
        use std::io::Cursor;

        let decoder = ImageReader::new(Cursor::new(ICON_PNG))
            .with_guessed_format()
            .map_err(|e| anyhow!("failed to read icon: {}", e))?
            .decode()
            .map_err(|e| anyhow!("failed to decode icon: {}", e))?;

        let rgba = decoder.to_rgba8();
        let (width, height) = (rgba.width() as i32, rgba.height() as i32);
        tracing::info!("icon width: {}, height: {}", width, height);

        // Status Notifier 使用 ARGB32 网络字节序：每像素 (A, R, G, B)
        let mut argb_data = Vec::with_capacity(rgba.len());
        for chunk in rgba.chunks_exact(4) {
            let [r, g, b, a] = chunk else { unreachable!() };
            //argb_data.extend_from_slice(&[*a, *r, *g, *b]);
            argb_data.extend_from_slice(&[*a, *r, *g, *b]);
        }

        Ok(IconSource::Data {
            width,
            height,
            data: argb_data,
        })
    }

    #[cfg(target_os = "windows")]
    {
        use image::ImageReader;
        use std::io::Cursor;
        use windows_sys::Win32::UI::WindowsAndMessaging::{CreateIcon, DestroyIcon};

        let img = ImageReader::new(Cursor::new(ICON_PNG))
            .with_guessed_format()
            .map_err(|e| anyhow!("failed to read icon: {}", e))?
            .decode()
            .map_err(|e| anyhow!("failed to decode icon: {}", e))?;

        let rgba_img = img.to_rgba8();
        let width = rgba_img.width() as i32;
        let height = rgba_img.height() as i32;
        let mut rgba = rgba_img.into_raw();
        let pixel_count = (width * height) as usize;

        // CreateIcon 需要：AND 遮罩（从 alpha 派生）+ BGRA 颜色数据
        // AND 遮罩：透明(alpha=0) -> 255，不透明(alpha=255) -> 0
        let mut and_mask = Vec::with_capacity(pixel_count);
        let pixels = unsafe {
            std::slice::from_raw_parts_mut(rgba.as_mut_ptr() as *mut [u8; 4], pixel_count)
        };
        for pixel in pixels {
            and_mask.push(pixel[3].wrapping_sub(u8::MAX)); // 反转 alpha
            pixel.swap(0, 2); // RGBA -> BGRA
        }

        let hicon = unsafe {
            CreateIcon(
                0_isize, // HINSTANCE: 0 表示无模块实例
                width,
                height,
                1,
                32,
                and_mask.as_ptr(),
                rgba.as_ptr(),
            )
        };

        if hicon == 0 {
            return Err(anyhow!("CreateIcon failed"));
        }

        Ok(IconSource::RawIcon(hicon))
    }

    #[cfg(target_os = "macos")]
    {
        use image::ImageReader;
        use std::io::Cursor;

        let decoder = ImageReader::new(Cursor::new(ICON_PNG))
            .with_guessed_format()
            .map_err(|e| anyhow!("failed to read icon: {}", e))?
            .decode()
            .map_err(|e| anyhow!("failed to decode icon: {}", e))?;

        let rgba = decoder.to_rgba8();
        let (width, height) = (rgba.width() as i32, rgba.height() as i32);
        let data = rgba.into_raw();

        Ok(IconSource::Data {
            width,
            height,
            data,
        })
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        Ok(IconSource::Resource(""))
    }
}

/// 托盘回调事件类型。
#[derive(Debug, Clone, PartialEq)]
pub enum TrayEvent {
    /// 退出程序
    Quit,
    /// 打开配置 UI
    OpenConfigUI,
    /// 复制配置路径（已废弃，保留兼容）
    OpenConfig,
}

/// 系统托盘管理器。
pub struct TrayManager {
    #[allow(dead_code)]
    tray: TrayItem,
    event_rx: mpsc::Receiver<TrayEvent>,
    shutdown: Arc<AtomicBool>,
    #[cfg(target_os = "windows")]
    icon_handle: windows_sys::Win32::Foundation::HICON,
}

#[cfg(target_os = "windows")]
impl Drop for TrayManager {
    fn drop(&mut self) {
        if self.icon_handle != 0 {
            unsafe {
                DestroyIcon(self.icon_handle);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl Drop for TrayManager {
    fn drop(&mut self) {}
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

        // 创建托盘项，使用内嵌图标（编译期嵌入，不依赖外部文件）
        let icon_source = embedded_tray_icon()?;
        let mut tray = TrayItem::new("LAN Clipboard Sync", icon_source)
            .map_err(|e| anyhow!("failed to create tray item: {}", e))?;

        // 添加不可点击的标题信息
        tray.add_label("LAN Clipboard Sync")
            .map_err(|e| anyhow!("failed to add title label: {}", e))?;
        tray.add_label(format!("Version: v{}", env!("CARGO_PKG_VERSION")).as_str())
            .map_err(|e| anyhow!("failed to add version label: {}", e))?;

        // 添加菜单项
        let event_tx_clone = event_tx.clone();
        tray.add_menu_item("配置", move || {
            tracing::info!("Config UI menu item clicked");
            let _ = event_tx_clone.send(TrayEvent::OpenConfigUI);
        })
        .map_err(|e| anyhow!("failed to add Config menu item: {}", e))?;

        let config_path_clone = config_path.clone();
        let event_tx_clone = event_tx.clone();
        tray.add_menu_item("复制配置路径", move || {
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

        #[cfg(target_os = "windows")]
        {
            let icon_handle = if let IconSource::RawIcon(hicon) = icon_source {
                hicon
            } else {
                0
            };
            Ok(Self {
                tray,
                event_rx,
                shutdown,
                icon_handle,
            })
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(Self {
                tray,
                event_rx,
                shutdown,
            })
        }
    }

    /// 尝试从托盘接收事件（非阻塞）。
    ///
    /// 返回 `Some(event)` 如果有事件可用，`None` 如果没有事件。
    pub fn try_recv(&self) -> Option<TrayEvent> {
        self.event_rx.try_recv().ok()
    }

    /// 阻塞等待托盘事件。
    pub fn recv(&self) -> Result<TrayEvent> {
        self.event_rx
            .recv()
            .map_err(|e| anyhow!("failed to receive tray event: {}", e))
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

        match crate::clipboard::write_text_to_clipboard(&dir_str) {
            Ok(()) => {
                tracing::info!("successfully copied to clipboard: {}", dir_str);
            }
            Err(e) => {
                tracing::error!("failed to copy to clipboard: {}", e);
            }
        }
    } else {
        tracing::error!("config file has no parent directory");
    }
}
