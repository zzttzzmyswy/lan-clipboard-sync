//! 核心业务逻辑：连接剪贴板抽象与网络层，实现去重与防回声的同步流程。

use crate::clipboard::{spawn_clipboard_watcher, ClipboardFile, ClipboardItem, SystemClipboard};
use crate::config::AppConfig;
use crate::network::{broadcast_to_peers, NetworkServer};
use crate::protocol::{ContentType, FileEntry, ProtocolMessage};
use anyhow::Result;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const SUPPRESS_WINDOW: Duration = Duration::from_millis(1500);

/// 核心服务：封装剪贴板监听、网络服务器与去重逻辑。
pub struct CoreService {
    config: AppConfig,
    instance_id: String,
    clipboard_change_rx: mpsc::Receiver<()>,
    incoming_msg_rx: mpsc::Receiver<ProtocolMessage>,
    _clipboard_watcher: JoinHandle<()>,
}

impl CoreService {
    /// 创建核心服务，启动剪贴板 watcher 与网络监听线程。
    pub fn new(config: AppConfig) -> Result<Self> {
        let (clip_tx, clip_rx) = mpsc::channel(32);
        let watcher = spawn_clipboard_watcher(clip_tx);

        let (incoming_tx, incoming_rx) = mpsc::channel(32);
        let server = NetworkServer::new(&config, incoming_tx)?;

        // 启动网络监听：单独线程内创建 Tokio runtime 运行异步服务器
        std::thread::spawn(move || {
            if let Ok(rt) = tokio::runtime::Runtime::new() {
                rt.block_on(async {
                    if let Err(e) = server.run().await {
                        tracing::error!("network server error: {e}");
                    }
                });
            } else {
                tracing::error!("failed to create tokio runtime for network server");
            }
        });

        let instance_id = config
            .instance_id
            .clone()
            .unwrap_or_else(|| hostname::get().unwrap_or_default().to_string_lossy().to_string());

        Ok(Self {
            config,
            instance_id,
            clipboard_change_rx: clip_rx,
            incoming_msg_rx: incoming_rx,
            _clipboard_watcher: watcher,
        })
    }

    /// 主事件循环：在本地剪贴板与远端更新之间做同步与去重。
    pub async fn run(&mut self) -> Result<()> {
        let mut clipboard = SystemClipboard::new()?;
        let mut last_hash: Option<u64> = None;
        // 远端写入后的屏蔽状态：记录写入时刻和写入内容的哈希
        let mut suppress_until: Option<Instant> = None;
        let mut suppress_hash: Option<u64> = None;
        tracing::debug!("clipboard sync started");

        loop {
            tokio::select! {
                Some(_) = self.clipboard_change_rx.recv() => {
                    tracing::debug!("clipboard changed");
                    // 检查是否在屏蔽窗口内
                    if let Some(deadline) = suppress_until {
                        if Instant::now() < deadline {
                            // 读取当前剪贴板内容，对比哈希
                            if let Some(item) = clipboard.read()? {
                                let h = hash_item(&item);
                                if h == suppress_hash {
                                    tracing::debug!("suppressed clipboard echo (within window, same hash)");
                                    continue;
                                }
                                // 哈希不同说明是真正的用户操作，清除屏蔽继续处理
                                tracing::debug!("hash mismatch during suppress window, treating as real change");
                            } else {
                                tracing::debug!("suppressed clipboard echo (within window, empty read)");
                                continue;
                            }
                        }
                        tracing::debug!("suppress window expired, clearing suppress state");
                        // 窗口已过期，清除屏蔽状态
                        suppress_until = None;
                        suppress_hash = None;
                    }

                    if let Some(item) = clipboard.read()? {
                        match &item {
                            ClipboardItem::Text(t) => {
                                tracing::info!("local clipboard changed: text len={}", t.len());
                            }
                            ClipboardItem::Image(bytes) => {
                                tracing::info!("local clipboard changed: image bytes={}", bytes.len());
                            }
                            ClipboardItem::Files(files) => {
                                tracing::info!("local clipboard changed: {} file(s)", files.len());
                            }
                        }
                        if let Some(h) = hash_item(&item) {
                            if last_hash == Some(h) {
                                continue;
                            }
                            last_hash = Some(h);
                        }
                        if let Some(msg) = self.build_clipboard_message(&item)? {
                            tracing::info!("broadcasting clipboard update to peers");
                            broadcast_to_peers(&self.config, &msg).await?;
                        }
                    }
                }
                Some(msg) = self.incoming_msg_rx.recv() => {
                    let ProtocolMessage::ClipboardUpdate { instance_id, content_type, payload_size: _, payload } = msg;
                    if instance_id == self.instance_id {
                        continue;
                    }
                    tracing::info!(
                        "received remote clipboard from instance_id={} type={:?} bytes={}",
                        instance_id,
                        content_type,
                        payload.len()
                    );
                    if let Some(item) = self.apply_remote_clipboard(content_type, &payload)? {
                        let written_hash = hash_item(&item);
                        suppress_until = Some(Instant::now() + SUPPRESS_WINDOW);
                        suppress_hash = written_hash;
                        // 同时更新 last_hash 避免后续重复广播
                        last_hash = written_hash;
                        tracing::debug!("set suppress window for {}ms", SUPPRESS_WINDOW.as_millis());
                        clipboard.write(item)?;
                    }
                }
                else => {
                    break;
                }
            }
        }
        Ok(())
    }

    /// 将当前剪贴板内容构造成要广播给所有 peers 的协议消息。
    fn build_clipboard_message(&self, item: &ClipboardItem) -> Result<Option<ProtocolMessage>> {
        match item {
            ClipboardItem::Text(text) => {
                let payload = text.as_bytes().to_vec();
                Ok(Some(ProtocolMessage::ClipboardUpdate {
                    instance_id: self.instance_id.clone(),
                    content_type: ContentType::Text,
                    payload_size: payload.len() as u64,
                    payload,
                }))
            }
            ClipboardItem::Image(png) => {
                let payload = png.clone();
                Ok(Some(ProtocolMessage::ClipboardUpdate {
                    instance_id: self.instance_id.clone(),
                    content_type: ContentType::Image,
                    payload_size: payload.len() as u64,
                    payload,
                }))
            }
            ClipboardItem::Files(files) => {
                let mut entries = Vec::new();
                for f in files {
                    let raw = &f.path;
                    // 剪贴板返回的路径可能带 file:// 前缀，需要去掉
                    let clean = if raw.starts_with("file://") {
                        &raw[7..]
                    } else {
                        raw.as_str()
                    };
                    // URL 编码的空格等字符需要解码
                    let decoded = percent_decode(clean);
                    let path = Path::new(&decoded);
                    tracing::debug!("reading file: raw={} resolved={}", raw, path.display());
                    let meta = match std::fs::metadata(path) {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::warn!("skip file {}: {}", path.display(), e);
                            continue;
                        }
                    };
                    if meta.is_dir() {
                        tracing::debug!("skip directory: {}", path.display());
                        continue;
                    }
                    let size = meta.len();
                    if size > self.config.max_file_size {
                        tracing::warn!("skip file {} larger than max_file_size", path.display());
                        return Ok(None);
                    }
                    let content = std::fs::read(path)?;
                    let name = path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "file".into());
                    entries.push(FileEntry { name, size, content });
                }
                if entries.is_empty() {
                    return Ok(None);
                }
                let payload = serde_json::to_vec(&entries)?;
                Ok(Some(ProtocolMessage::ClipboardUpdate {
                    instance_id: self.instance_id.clone(),
                    content_type: ContentType::Files,
                    payload_size: payload.len() as u64,
                    payload,
                }))
            }
        }
    }

    /// 将远端收到的协议消息解析并落地成本机剪贴板条目（文件会写入下载目录）。
    fn apply_remote_clipboard(
        &self,
        content_type: ContentType,
        payload: &[u8],
    ) -> Result<Option<ClipboardItem>> {
        match content_type {
            ContentType::Text => {
                let text = String::from_utf8(payload.to_vec())?;
                Ok(Some(ClipboardItem::Text(text)))
            }
            ContentType::Image => Ok(Some(ClipboardItem::Image(payload.to_vec()))),
            ContentType::Files => {
                let entries: Vec<FileEntry> = serde_json::from_slice(payload)?;
                let base = self.download_dir();
                std::fs::create_dir_all(&base)?;
                let mut files = Vec::new();
                for e in entries {
                    let path = base.join(&e.name);
                    std::fs::write(&path, &e.content)?;
                    files.push(ClipboardFile {
                        path: path.to_string_lossy().to_string(),
                    });
                }
                Ok(Some(ClipboardItem::Files(files)))
            }
        }
    }

    /// 返回用于保存远端文件的下载目录，按平台选择合适的 `Downloads` 路径。
    fn download_dir(&self) -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home).join("Downloads").join("lan-clipboard");
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(home) = std::env::var_os("USERPROFILE") {
                return PathBuf::from(home).join("Downloads").join("lan-clipboard");
            }
        }
        PathBuf::from("lan-clipboard-downloads")
    }

}

/// 简易 percent-decode：将 `%XX` 序列还原为原始字节并转回 UTF-8 字符串。
fn percent_decode(input: &str) -> String {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(
                &input[i + 1..i + 3],
                16,
            ) {
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

/// 根据剪贴板内容计算粗粒度哈希，用于去重与抑制回环更新。
fn hash_item(item: &ClipboardItem) -> Option<u64> {
    let mut hasher = DefaultHasher::new();
    match item {
        ClipboardItem::Text(t) => t.hash(&mut hasher),
        ClipboardItem::Image(bytes) => bytes.hash(&mut hasher),
        ClipboardItem::Files(files) => {
            "files".hash(&mut hasher);
            for f in files {
                // 只用文件名（不含目录）+ 文件大小来算哈希，
                // 这样发送端（file:///原始/路径/foo.txt）和
                // 接收端（/Downloads/lan-clipboard/foo.txt）能得到相同结果
                let p = std::path::Path::new(&f.path);
                let clean = if f.path.starts_with("file://") {
                    std::path::Path::new(&f.path[7..])
                } else {
                    p
                };
                if let Some(name) = clean.file_name() {
                    name.hash(&mut hasher);
                }
                if let Ok(meta) = std::fs::metadata(clean) {
                    meta.len().hash(&mut hasher);
                }
            }
        }
    }
    Some(hasher.finish())
}

