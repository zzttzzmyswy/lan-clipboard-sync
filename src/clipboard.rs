//! 剪贴板抽象与系统集成封装：统一文本、图片与文件的读写与变更监听。

use anyhow::{anyhow, Result};
use clipboard_rs::common::{ClipboardHandler, ContentFormat, RustImage, RustImageData};
use clipboard_rs::{Clipboard, ClipboardContext, ClipboardWatcher, ClipboardWatcherContext};
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
    Image(Vec<u8>),          // PNG 字节
    Files(Vec<ClipboardFile>),
}

/// 系统剪贴板读写封装
pub struct SystemClipboard {
    ctx: ClipboardContext,
}

impl SystemClipboard {
    pub fn new() -> Result<Self> {
        let ctx = ClipboardContext::new().map_err(|e| anyhow!(e.to_string()))?;
        Ok(Self { ctx })
    }

    /// 读取当前剪贴板内容（按 Files > Image > Text 优先级）
    pub fn read(&self) -> Result<Option<ClipboardItem>> {
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

    /// 将内容写入系统剪贴板
    pub fn write(&mut self, item: ClipboardItem) -> Result<()> {
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
                    .map_err(|e| anyhow!(e.to_string()))?;
            }
            ClipboardItem::Files(files) => {
                let count = files.len();
                let uris: Vec<String> = files.into_iter().map(|f| f.path).collect();
                tracing::info!("clipboard write: {} file(s)", count);
                self.ctx
                    .set_files(uris)
                    .map_err(|e| anyhow!(e.to_string()))?;
            }
        }
        Ok(())
    }
}

/// 剪贴板变化 watcher，向通道发送简单事件
pub fn spawn_clipboard_watcher(tx: mpsc::Sender<()>) -> thread::JoinHandle<()> {
    struct Handler {
        tx: mpsc::Sender<()>,
    }

    impl ClipboardHandler for Handler {
        fn on_clipboard_change(&mut self) {
            tracing::debug!("clipboard watcher: change detected");
            let _ = self.tx.try_send(());
        }
    }

    thread::spawn(move || {
        match ClipboardWatcherContext::<Handler>::new() {
            Ok(mut watcher) => {
                tracing::info!("clipboard watcher started");
                watcher.add_handler(Handler { tx });
                watcher.start_watch();
                tracing::warn!("clipboard watcher exited");
            }
            Err(e) => {
                tracing::error!("clipboard watcher failed to start: {}", e);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_item_debug() {
        let _ = format!("{:?}", ClipboardItem::Text("x".into()));
    }
}

