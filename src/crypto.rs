//! 加密工具模块：X25519 密钥交换 + HKDF 会话密钥派生 + ChaCha20-Poly1305 加解密。

use anyhow::{anyhow, Result};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey};

/// HKDF 的 info 参数，用于绑定会话密钥用途
const HKDF_INFO: &[u8] = b"lan-clipboard-sync-v1";

/// 从十六进制字符串生成对称密钥（用作 PSK，参与密钥派生）
pub fn key_from_hex(hex: &str) -> Result<Key> {
    let bytes = hex::decode(hex)?;
    if bytes.len() != 32 {
        return Err(anyhow!("secret_key must be 32 bytes after decoding"));
    }
    Ok(Key::from_slice(&bytes).to_owned())
}

/// 从 ECDH 共享密钥与 PSK 派生出 32 字节会话密钥（用于 ChaCha20-Poly1305）
fn derive_session_key(shared_secret: &[u8], psk: &[u8; 32]) -> Key {
    let hk = Hkdf::<Sha256>::new(Some(psk), shared_secret);
    let mut key = [0u8; 32];
    hk.expand(HKDF_INFO, &mut key)
        .expect("HKDF expand 32 bytes");
    Key::from_slice(&key).to_owned()
}

/// 生成随机 nonce（12 字节）
pub fn random_nonce() -> [u8; 12] {
    let mut bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

/// 加密：返回 (nonce_bytes, ciphertext)
pub fn encrypt(key: &Key, plaintext: &[u8]) -> Result<([u8; 12], Vec<u8>)> {
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = random_nonce();
    let nonce_ref = Nonce::from_slice(&nonce);
    let ct = cipher
        .encrypt(nonce_ref, plaintext)
        .map_err(|e| anyhow!("encrypt failed: {e}"))?;
    Ok((nonce, ct))
}

/// 解密：传入 nonce 与密文
pub fn decrypt(key: &Key, nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(key);
    let nonce_ref = Nonce::from_slice(nonce);
    let pt = cipher
        .decrypt(nonce_ref, ciphertext)
        .map_err(|e| anyhow!("decrypt failed: {e}"))?;
    Ok(pt)
}

/// 密钥交换公钥长度（X25519）
pub const PUBLIC_KEY_LEN: usize = 32;

/// 客户端握手：发送本端公钥，接收对端公钥，完成 ECDH 并派生会话密钥。
pub async fn handshake_client<S>(stream: &mut S, psk: &[u8; 32]) -> Result<Key>
where
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Unpin,
{
    let secret = EphemeralSecret::random_from_rng(rand::thread_rng());
    let public = PublicKey::from(&secret);
    stream.write_all(public.as_bytes()).await?;
    stream.flush().await?;
    let mut peer_bytes = [0u8; PUBLIC_KEY_LEN];
    stream.read_exact(&mut peer_bytes).await?;
    let peer = PublicKey::from(peer_bytes);
    let shared = secret.diffie_hellman(&peer);
    Ok(derive_session_key(shared.as_bytes(), psk))
}

/// 服务端握手：接收客户端公钥，发送本端公钥，完成 ECDH 并派生会话密钥。
pub async fn handshake_server<S>(stream: &mut S, psk: &[u8; 32]) -> Result<Key>
where
    S: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Unpin,
{
    let mut peer_bytes = [0u8; PUBLIC_KEY_LEN];
    stream.read_exact(&mut peer_bytes).await?;
    let secret = EphemeralSecret::random_from_rng(rand::thread_rng());
    let public = PublicKey::from(&secret);
    stream.write_all(public.as_bytes()).await?;
    stream.flush().await?;
    let peer = PublicKey::from(peer_bytes);
    let shared = secret.diffie_hellman(&peer);
    Ok(derive_session_key(shared.as_bytes(), psk))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let key = key_from_hex(key_hex).unwrap();
        let msg = b"hello world";
        let (nonce, ct) = encrypt(&key, msg).unwrap();
        let pt = decrypt(&key, &nonce, &ct).unwrap();
        assert_eq!(&pt, msg);
    }
}

