//! 网络传输层：基于 TCP + 对称加密的剪贴板消息收发。

use crate::config::AppConfig;
use crate::crypto::{decrypt, encrypt, key_from_hex};
use crate::protocol::{decode_message, encode_frame, encode_message, ProtocolMessage};
use anyhow::{anyhow, Result};
use chacha20poly1305::Key;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// 入站帧体的最大字节数（约 50 MiB），防止恶意/异常连接导致 OOM
const MAX_FRAME_BODY: usize = 50 * 1024 * 1024;

/// 入站连接读超时，防止慢速连接占用资源
const CONNECTION_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// 网络层：负责监听远端连接并将解密后的消息推送到核心逻辑。
pub struct NetworkServer {
    addr: SocketAddr,
    key: Key,
    incoming_tx: mpsc::Sender<ProtocolMessage>,
}

impl NetworkServer {
    pub fn new(config: &AppConfig, incoming_tx: mpsc::Sender<ProtocolMessage>) -> Result<Self> {
        let key = key_from_hex(&config.secret_key)?;
        let addr = SocketAddr::new(IpAddr::from([0, 0, 0, 0]), config.listen_port);
        Ok(Self { addr, key, incoming_tx })
    }

    /// 启动 TCP 监听循环，为每个入站连接创建异步任务。
    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        loop {
            let (stream, _) = listener.accept().await?;
            let key = self.key.clone();
            let tx = self.incoming_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, key, tx).await {
                    tracing::warn!("connection error: {e}");
                }
            });
        }
    }
}

/// 处理单个入站 TCP 连接：读取、解密并解码协议消息后发送到通道。
/// 带帧长度上限校验和读超时，防止 OOM 与资源耗尽。
async fn handle_connection(
    mut stream: TcpStream,
    key: Key,
    incoming_tx: mpsc::Sender<ProtocolMessage>,
) -> Result<()> {
    let read_ops = async {
        // 先读取 4 字节长度
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        if len > MAX_FRAME_BODY {
            return Err(anyhow!(
                "frame body too large: {} > {} bytes",
                len,
                MAX_FRAME_BODY
            ));
        }

        let mut body = vec![0u8; len];
        stream.read_exact(&mut body).await?;

        if body.len() < 12 {
            return Err(anyhow!("frame body too short for nonce"));
        }
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&body[..12]);
        let ciphertext = &body[12..];
        let plaintext = decrypt(&key, &nonce, ciphertext)?;
        let msg = decode_message(&plaintext)?;
        incoming_tx.send(msg).await.map_err(|_| anyhow!("channel closed"))?;
        Ok(())
    };

    tokio::time::timeout(CONNECTION_READ_TIMEOUT, read_ops)
        .await
        .map_err(|_| anyhow!("connection read timeout"))??;
    Ok(())
}

/// 将剪贴板更新消息加密后广播到配置中的所有 peers（2秒超时，并行执行）。
pub async fn broadcast_to_peers(config: &AppConfig, msg: &ProtocolMessage) -> Result<()> {
    let key = key_from_hex(&config.secret_key)?;
    let body = encode_message(msg)?;
    let (nonce, ciphertext) = encrypt(&key, &body)?;

    let mut frame_body = Vec::with_capacity(12 + ciphertext.len());
    frame_body.extend_from_slice(&nonce);
    frame_body.extend_from_slice(&ciphertext);
    let frame = encode_frame(&frame_body);

    let timeout_duration = Duration::from_secs(2);
    let mut tasks = Vec::new();

    for peer in &config.peers {
        let addr = format!("{}:{}", peer.host, peer.port);
        let frame_clone = frame.clone();
        let addr_clone = addr.clone();

        let task = tokio::spawn(async move {
            let result = tokio::time::timeout(timeout_duration, async {
                let mut stream = TcpStream::connect(&addr_clone).await?;
                stream.write_all(&frame_clone).await?;
                Ok::<_, anyhow::Error>(())
            })
            .await;

            match result {
                Ok(Ok(())) => {
                    tracing::debug!("successfully sent to {addr_clone}");
                }
                Ok(Err(e)) => {
                    tracing::warn!("send to {addr_clone} failed: {e}");
                }
                Err(_) => {
                    tracing::debug!("send to {addr_clone} timed out after 2s");
                }
            }
        });
        tasks.push(task);
    }

    for task in tasks {
        let _ = task.await;
    }

    Ok(())
}

