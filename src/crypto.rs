//! 加密工具模块：封装密钥派生、随机 nonce 生成与对称加解密接口。

use anyhow::{anyhow, Result};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;

/// 从十六进制字符串生成对称密钥
pub fn key_from_hex(hex: &str) -> Result<Key> {
    let bytes = hex::decode(hex)?;
    if bytes.len() != 32 {
        return Err(anyhow!("secret_key must be 32 bytes after decoding"));
    }
    Ok(Key::from_slice(&bytes).to_owned())
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

