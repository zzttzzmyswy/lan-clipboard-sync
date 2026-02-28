//! 剪贴板抽象与系统集成封装：统一文本、图片与文件的读写与变更监听。
//!
//! Linux 下根据 WAYLAND_DISPLAY 环境变量自动选择：
//! - Wayland: 使用 wl-clipboard-rs（参考 smithay_clipboard 的 Wayland 剪贴板方案）
//! - X11: 使用 clipboard-rs

use anyhow::{anyhow, Result};
use clipboard_rs::common::RustImage;
use clipboard_rs::Clipboard;
use std::thread;
use tokio::sync::mpsc;

/// 表示文件型剪贴板条目（仅保存路径，由上层负责读取内容与大小判断）
#[derive(Debug, Clone)]
pub struct ClipboardFile {
    pub path: String,
}

/// 统一的剪贴板内容抽象
#[derive(Debug, Clone)]
pub enum ClipboardItem {
    Text(String),
    Image(Vec<u8>), // PNG 字节
    Files(Vec<ClipboardFile>),
}

/// Linux 下检测是否为 Wayland 环境
#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// 系统剪贴板读写封装
pub struct SystemClipboard {
    #[cfg(target_os = "linux")]
    backend: LinuxClipboardBackend,
    #[cfg(not(target_os = "linux"))]
    backend: ClipboardRsBackend,
}

#[cfg(target_os = "linux")]
enum LinuxClipboardBackend {
    Wayland(WaylandClipboardBackend),
    X11(ClipboardRsBackend),
}

/// clipboard-rs 后端（X11 / Windows）
struct ClipboardRsBackend {
    ctx: clipboard_rs::ClipboardContext,
}

#[cfg(target_os = "linux")]
/// wl-clipboard-rs 后端（Wayland，参考 smithay_clipboard 的 Wayland 剪贴板方案）
struct WaylandClipboardBackend;

impl SystemClipboard {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            let backend = if is_wayland() {
                tracing::info!("using Wayland clipboard backend (wl-clipboard-rs)");
                LinuxClipboardBackend::Wayland(WaylandClipboardBackend)
            } else {
                tracing::info!("using X11 clipboard backend (clipboard-rs)");
                let ctx =
                    clipboard_rs::ClipboardContext::new().map_err(|e| anyhow!(e.to_string()))?;
                LinuxClipboardBackend::X11(ClipboardRsBackend { ctx })
            };
            Ok(Self { backend })
        }

        #[cfg(not(target_os = "linux"))]
        {
            let ctx = clipboard_rs::ClipboardContext::new().map_err(|e| anyhow!(e.to_string()))?;
            Ok(Self {
                backend: ClipboardRsBackend { ctx },
            })
        }
    }

    /// 读取当前剪贴板内容（按 Files > Image > Text 优先级）
    pub fn read(&self) -> Result<Option<ClipboardItem>> {
        #[cfg(target_os = "linux")]
        match &self.backend {
            LinuxClipboardBackend::Wayland(w) => w.read(),
            LinuxClipboardBackend::X11(x) => x.read(),
        }

        #[cfg(not(target_os = "linux"))]
        self.backend.read()
    }

    /// 将内容写入系统剪贴板
    pub fn write(&mut self, item: ClipboardItem) -> Result<()> {
        #[cfg(target_os = "linux")]
        match &mut self.backend {
            LinuxClipboardBackend::Wayland(w) => w.write(item),
            LinuxClipboardBackend::X11(x) => x.write(item),
        }

        #[cfg(not(target_os = "linux"))]
        self.backend.write(item)
    }
}

impl ClipboardRsBackend {
    fn read(&self) -> Result<Option<ClipboardItem>> {
        use clipboard_rs::common::ContentFormat;

        // 文件
        if self.ctx.has(ContentFormat::Files) {
            let files = self.ctx.get_files().unwrap_or_default();
            if !files.is_empty() {
                let items: Vec<ClipboardFile> = files
                    .into_iter()
                    .map(|p| ClipboardFile { path: p })
                    .collect();
                tracing::debug!("clipboard read: {} file(s)", items.len());
                return Ok(Some(ClipboardItem::Files(items)));
            }
        }

        // 图片
        if self.ctx.has(ContentFormat::Image) {
            if let Ok(formats) = self.ctx.available_formats() {
                if let Some(fmt) = formats
                    .iter()
                    .find(|f| f.contains("image") || f.contains("png"))
                {
                    if let Ok(buf) = self.ctx.get_buffer(fmt) {
                        tracing::debug!(
                            "clipboard read: image buffer len={} format={}",
                            buf.len(),
                            fmt
                        );
                        return Ok(Some(ClipboardItem::Image(buf)));
                    }
                }
            }
        }

        // 文本
        if self.ctx.has(ContentFormat::Text) {
            if let Ok(text) = self.ctx.get_text() {
                if !text.is_empty() {
                    tracing::debug!("clipboard read: text len={}", text.len());
                    return Ok(Some(ClipboardItem::Text(text)));
                }
            }
        }

        Ok(None)
    }

    fn write(&mut self, item: ClipboardItem) -> Result<()> {
        use clipboard_rs::common::RustImageData;

        match item {
            ClipboardItem::Text(text) => {
                tracing::info!("clipboard write: text len={}", text.len());
                self.ctx
                    .set_text(text)
                    .map_err(|e| anyhow!(e.to_string()))?
            }
            ClipboardItem::Image(png_bytes) => {
                tracing::info!("clipboard write: image bytes={}", png_bytes.len());
                let img =
                    RustImageData::from_bytes(&png_bytes).map_err(|e| anyhow!(e.to_string()))?;
                self.ctx
                    .set_image(img)
                    .map_err(|e| anyhow!(e.to_string()))?
            }
            ClipboardItem::Files(files) => {
                let count = files.len();
                let uris: Vec<String> = files.into_iter().map(|f| f.path).collect();
                tracing::info!("clipboard write: {} file(s)", count);
                self.ctx
                    .set_files(uris)
                    .map_err(|e| anyhow!(e.to_string()))?
            }
        }
        Ok(())
    }
}

// 修复 ClipboardRsBackend 的 read 中误用 ClipboardHandler
#[cfg(target_os = "linux")]
impl WaylandClipboardBackend {
    fn read(&self) -> Result<Option<ClipboardItem>> {
        use std::io::Read;
        use wl_clipboard_rs::paste::{
            get_contents, get_mime_types, ClipboardType, Error, MimeType, Seat,
        };

        let mime_types = match get_mime_types(ClipboardType::Regular, Seat::Unspecified) {
            Ok(m) => m,
            Err(Error::NoSeats) | Err(Error::ClipboardEmpty) => return Ok(None),
            Err(Error::MissingProtocol { .. }) => return Ok(None),
            Err(e) => return Err(anyhow!("wayland clipboard read: {}", e)),
        };

        // 优先级: text/uri-list (文件) > image/* > text
        if mime_types.contains("text/uri-list") {
            if let Ok((mut pipe, _)) = get_contents(
                ClipboardType::Regular,
                Seat::Unspecified,
                MimeType::Specific("text/uri-list"),
            ) {
                let mut buf = Vec::new();
                if pipe.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                    let uri_list = String::from_utf8_lossy(&buf);
                    let files: Vec<ClipboardFile> = uri_list
                        .lines()
                        .filter_map(|line| {
                            let line = line.trim();
                            if line.is_empty() || line.starts_with('#') {
                                return None;
                            }
                            let path = if let Some(stripped) = line.strip_prefix("file://") {
                                url_decode(stripped)
                            } else {
                                line.to_string()
                            };
                            if !path.is_empty() {
                                Some(ClipboardFile { path })
                            } else {
                                None
                            }
                        })
                        .collect();
                    if !files.is_empty() {
                        tracing::debug!("wayland clipboard read: {} file(s)", files.len());
                        return Ok(Some(ClipboardItem::Files(files)));
                    }
                }
            }
        }

        // 图片: 尝试 image/png
        let image_mime = mime_types
            .iter()
            .find(|m| m.starts_with("image/png"))
            .map(|s| s.as_str());
        if let Some(mime) = image_mime {
            if let Ok((mut pipe, _)) = get_contents(
                ClipboardType::Regular,
                Seat::Unspecified,
                MimeType::Specific(mime),
            ) {
                let mut buf = Vec::new();
                if pipe.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                    tracing::debug!("wayland clipboard read: image bytes={}", buf.len());
                    return Ok(Some(ClipboardItem::Image(buf)));
                }
            }
        }

        // 文本
        match get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text) {
            Ok((mut pipe, _)) => {
                let mut buf = Vec::new();
                if pipe.read_to_end(&mut buf).is_ok() {
                    if let Ok(text) = String::from_utf8(buf) {
                        if !text.is_empty() {
                            tracing::debug!("wayland clipboard read: text len={}", text.len());
                            return Ok(Some(ClipboardItem::Text(text)));
                        }
                    }
                }
            }
            Err(Error::NoSeats) | Err(Error::ClipboardEmpty) | Err(Error::NoMimeType) => {}
            Err(e) => return Err(anyhow!("wayland clipboard text read: {}", e)),
        }

        Ok(None)
    }

    fn write(&self, item: ClipboardItem) -> Result<()> {
        use wl_clipboard_rs::copy::{MimeType, Options, Source};

        let opts = Options::new();
        match item {
            ClipboardItem::Text(text) => {
                tracing::info!("wayland clipboard write: text len={}", text.len());
                opts.copy(
                    Source::Bytes(text.into_bytes().into_boxed_slice()),
                    MimeType::Text,
                )
                .map_err(|e| anyhow!("wayland clipboard write: {}", e))?;
            }
            ClipboardItem::Image(png_bytes) => {
                tracing::info!("wayland clipboard write: image bytes={}", png_bytes.len());
                opts.copy(
                    Source::Bytes(png_bytes.into_boxed_slice()),
                    MimeType::Specific("image/png".to_string()),
                )
                .map_err(|e| anyhow!("wayland clipboard write: {}", e))?;
            }
            ClipboardItem::Files(files) => {
                let uri_list: String = files
                    .iter()
                    .map(|f| {
                        let p = &f.path;
                        if p.starts_with("file://") {
                            p.clone()
                        } else {
                            format!("file://{}", p)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\r\n");
                tracing::info!("wayland clipboard write: {} file(s)", files.len());
                opts.copy(
                    Source::Bytes(uri_list.into_bytes().into_boxed_slice()),
                    MimeType::Specific("text/uri-list".to_string()),
                )
                .map_err(|e| anyhow!("wayland clipboard write: {}", e))?;
            }
        }
        Ok(())
    }
}

/// 简易 URI 解码（file:// 路径可能含 %XX）
fn url_decode(input: &str) -> String {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(val);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 剪贴板变化 watcher，向通道发送简单事件
/// - X11/Windows: 使用 clipboard-rs 的原生监听
/// - Wayland: 使用轮询（wl-clipboard-rs 无原生监听接口）
pub fn spawn_clipboard_watcher(tx: mpsc::Sender<()>) -> thread::JoinHandle<()> {
    #[cfg(target_os = "linux")]
    {
        if is_wayland() {
            return spawn_wayland_clipboard_watcher(tx);
        }
    }

    spawn_clipboard_rs_watcher(tx)
}

/// clipboard-rs 原生 watcher（X11/Windows）
fn spawn_clipboard_rs_watcher(tx: mpsc::Sender<()>) -> thread::JoinHandle<()> {
    use clipboard_rs::common::ClipboardHandler;
    use clipboard_rs::{ClipboardWatcher, ClipboardWatcherContext};

    struct Handler {
        tx: mpsc::Sender<()>,
    }

    impl ClipboardHandler for Handler {
        fn on_clipboard_change(&mut self) {
            tracing::debug!("clipboard watcher: change detected");
            let _ = self.tx.try_send(());
        }
    }

    thread::spawn(move || match ClipboardWatcherContext::<Handler>::new() {
        Ok(mut watcher) => {
            tracing::info!("clipboard watcher started (clipboard-rs)");
            watcher.add_handler(Handler { tx });
            watcher.start_watch();
            tracing::warn!("clipboard watcher exited");
        }
        Err(e) => {
            tracing::error!("clipboard watcher failed to start: {}", e);
        }
    })
}

#[cfg(target_os = "linux")]
/// Wayland 剪贴板轮询 watcher（wl-clipboard-rs 无原生监听，采用轮询）
fn spawn_wayland_clipboard_watcher(tx: mpsc::Sender<()>) -> thread::JoinHandle<()> {
    use std::time::Duration;

    thread::spawn(move || {
        const POLL_INTERVAL: Duration = Duration::from_millis(500);

        let mut last_hash: Option<u64> = None;
        tracing::info!("clipboard watcher started (Wayland polling)");

        loop {
            std::thread::sleep(POLL_INTERVAL);

            let current = match read_wayland_for_watcher() {
                Some(item) => hash_clipboard_item(&item),
                None => None,
            };

            if current != last_hash {
                last_hash = current;
                let _ = tx.try_send(());
            }
        }
    })
}

#[cfg(target_os = "linux")]
fn read_wayland_for_watcher() -> Option<ClipboardItem> {
    use std::io::Read;
    use wl_clipboard_rs::paste::{
        get_contents, get_mime_types, ClipboardType, Error, MimeType, Seat,
    };

    let mime_types = get_mime_types(ClipboardType::Regular, Seat::Unspecified).ok()?;

    if mime_types.contains("text/uri-list") {
        if let Ok((mut pipe, _)) = get_contents(
            ClipboardType::Regular,
            Seat::Unspecified,
            MimeType::Specific("text/uri-list"),
        ) {
            let mut buf = Vec::new();
            if pipe.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                let uri_list = String::from_utf8_lossy(&buf);
                let files: Vec<ClipboardFile> = uri_list
                    .lines()
                    .filter_map(|line| {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            return None;
                        }
                        let path = if let Some(stripped) = line.strip_prefix("file://") {
                            url_decode(stripped)
                        } else {
                            line.to_string()
                        };
                        if !path.is_empty() {
                            Some(ClipboardFile { path })
                        } else {
                            None
                        }
                    })
                    .collect();
                if !files.is_empty() {
                    return Some(ClipboardItem::Files(files));
                }
            }
        }
    }

    let image_mime = mime_types
        .iter()
        .find(|m| m.starts_with("image/png"))
        .map(|s| s.as_str())
        .or_else(|| mime_types.iter().find(|m| m.starts_with("image/")).map(|s| s.as_str()));
    if let Some(mime) = image_mime {
        if let Ok((mut pipe, _)) = get_contents(
            ClipboardType::Regular,
            Seat::Unspecified,
            MimeType::Specific(mime),
        ) {
            let mut buf = Vec::new();
            if pipe.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                return Some(ClipboardItem::Image(buf));
            }
        }
    }

    match get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text) {
        Ok((mut pipe, _)) => {
            let mut buf = Vec::new();
            if pipe.read_to_end(&mut buf).is_ok() {
                if let Ok(text) = String::from_utf8(buf) {
                    if !text.is_empty() {
                        return Some(ClipboardItem::Text(text));
                    }
                }
            }
        }
        Err(Error::NoSeats) | Err(Error::ClipboardEmpty) | Err(Error::NoMimeType) => {}
        Err(_) => {}
    }

    None
}

#[cfg(target_os = "linux")]
fn hash_clipboard_item(item: &ClipboardItem) -> Option<u64> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    match item {
        ClipboardItem::Text(t) => t.hash(&mut hasher),
        ClipboardItem::Image(bytes) => bytes.hash(&mut hasher),
        ClipboardItem::Files(files) => {
            "files".hash(&mut hasher);
            for f in files {
                let p = std::path::Path::new(&f.path);
                if let Some(name) = p.file_name() {
                    name.hash(&mut hasher);
                }
                if let Ok(meta) = std::fs::metadata(p) {
                    meta.len().hash(&mut hasher);
                }
            }
        }
    }
    Some(hasher.finish())
}

/// 将文本写入剪贴板（供托盘等模块使用，自动选择后端）
pub fn write_text_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = SystemClipboard::new()?;
    clipboard.write(ClipboardItem::Text(text.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_item_debug() {
        let _ = format!("{:?}", ClipboardItem::Text("x".into()));
    }
}
