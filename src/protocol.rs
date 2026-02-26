use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// 剪贴板内容类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ContentType {
    Text = 1,
    Image = 2,
    Files = 3,
}

impl TryFrom<u8> for ContentType {
    type Error = anyhow::Error;

    fn try_from(v: u8) -> Result<Self> {
        match v {
            1 => Ok(ContentType::Text),
            2 => Ok(ContentType::Image),
            3 => Ok(ContentType::Files),
            _ => Err(anyhow!("unknown content type {}", v)),
        }
    }
}

/// 单个文件条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
    pub content: Vec<u8>,
}

/// 协议消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolMessage {
    ClipboardUpdate {
        content_type: ContentType,
        payload_size: u64,
        payload: Vec<u8>,
    },
}

const VERSION: u8 = 1;
const MSG_TYPE_CLIPBOARD: u8 = 1;

/// 将 ProtocolMessage 编码为未加密的字节流
pub fn encode_message(msg: &ProtocolMessage) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    buf.push(VERSION);
    match msg {
        ProtocolMessage::ClipboardUpdate {
            content_type,
            payload_size,
            payload,
        } => {
            buf.push(MSG_TYPE_CLIPBOARD);
            buf.push(*content_type as u8);
            buf.extend_from_slice(&payload_size.to_be_bytes());
            buf.extend_from_slice(payload);
        }
    }
    Ok(buf)
}

/// 从未加密的字节流解码 ProtocolMessage
pub fn decode_message(mut data: &[u8]) -> Result<ProtocolMessage> {
    if data.len() < 2 {
        return Err(anyhow!("message too short"));
    }
    let version = data[0];
    if version != VERSION {
        return Err(anyhow!("unsupported version {}", version));
    }
    let msg_type = data[1];
    data = &data[2..];

    match msg_type {
        MSG_TYPE_CLIPBOARD => {
            if data.len() < 1 + 8 {
                return Err(anyhow!("message too short for body"));
            }
            let content_type = ContentType::try_from(data[0])?;
            data = &data[1..];
            let mut sz_bytes = [0u8; 8];
            sz_bytes.copy_from_slice(&data[..8]);
            let payload_size = u64::from_be_bytes(sz_bytes);
            data = &data[8..];
            let payload = data.to_vec();
            Ok(ProtocolMessage::ClipboardUpdate {
                content_type,
                payload_size,
                payload,
            })
        }
        _ => Err(anyhow!("unknown message type {}", msg_type)),
    }
}

/// 长度前缀帧编码：u32(长度) + 负载
pub fn encode_frame(body: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + body.len());
    let len = body.len() as u32;
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(body);
    buf
}

/// 从缓冲中尝试解出一帧（不消费多余字节）
pub fn try_decode_frame(buf: &[u8]) -> Option<(usize, Vec<u8>)> {
    if buf.len() < 4 {
        return None;
    }
    let mut len_bytes = [0u8; 4];
    len_bytes.copy_from_slice(&buf[..4]);
    let len = u32::from_be_bytes(len_bytes) as usize;
    if buf.len() < 4 + len {
        return None;
    }
    let body = buf[4..4 + len].to_vec();
    Some((4 + len, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let msg = ProtocolMessage::ClipboardUpdate {
            content_type: ContentType::Text,
            payload_size: 5,
            payload: b"hello".to_vec(),
        };
        let bytes = encode_message(&msg).unwrap();
        let decoded = decode_message(&bytes).unwrap();
        match decoded {
            ProtocolMessage::ClipboardUpdate {
                content_type,
                payload_size,
                payload,
            } => {
                assert!(matches!(content_type, ContentType::Text));
                assert_eq!(payload_size, 5);
                assert_eq!(payload, b"hello");
            }
        }
    }

    #[test]
    fn frame_roundtrip() {
        let body = vec![1, 2, 3, 4, 5];
        let framed = encode_frame(&body);
        let (used, decoded) = try_decode_frame(&framed).unwrap();
        assert_eq!(used, framed.len());
        assert_eq!(decoded, body);
    }
}

