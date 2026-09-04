//! 密文格式：`MX1.` + base64(iv[12] || ciphertext || tag[16])
//! AES-256-GCM，密码经 sha256 摊成 32 字节密钥（任意长度都行）。

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::RngCore;
use sha2::{Digest, Sha256};

pub const PREFIX: &str = "MX1.";

pub fn encrypt(password: &str, plain: &str) -> Result<String, String> {
    let cipher = cipher_from_password(password);
    let mut iv = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut iv);
    let ct = cipher
        .encrypt(Nonce::from_slice(&iv), plain.as_bytes())
        .map_err(|_| "加密失败".to_string())?;
    let mut blob = Vec::with_capacity(12 + ct.len());
    blob.extend_from_slice(&iv);
    blob.extend_from_slice(&ct);
    Ok(format!("{PREFIX}{}", STANDARD.encode(blob)))
}

pub fn decrypt(password: &str, token: &str) -> Result<String, String> {
    let compact: String = token.chars().filter(|c| !c.is_whitespace()).collect();
    let body = compact.strip_prefix(PREFIX).unwrap_or(&compact);
    let blob = STANDARD
        .decode(body)
        .map_err(|_| "这不是密信（乱码格式不对，检查是不是整行都贴进来了）".to_string())?;
    if blob.len() < 12 + 16 {
        return Err("这不是密信（内容太短了）".into());
    }
    let (iv, rest) = blob.split_at(12);
    let cipher = cipher_from_password(password);
    let pt = cipher
        .decrypt(Nonce::from_slice(iv), rest)
        .map_err(|_| "密码不对，或内容被改过。".to_string())?;
    String::from_utf8(pt).map_err(|_| "解密出来不是文字".into())
}

fn cipher_from_password(password: &str) -> Aes256Gcm {
    let hash = Sha256::digest(password.as_bytes());
    Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let t = encrypt("口令", "hello 解密测试 ✅").unwrap();
        assert!(t.starts_with(PREFIX));
        assert_eq!(decrypt("口令", &t).unwrap(), "hello 解密测试 ✅");
        assert_eq!(
            decrypt("口令", &format!("  {t}\n")).unwrap(),
            "hello 解密测试 ✅"
        );
        assert!(decrypt("错密码", &t).is_err());
    }

    #[test]
    fn random_iv_and_tamper() {
        let a = encrypt("p", "same").unwrap();
        let b = encrypt("p", "same").unwrap();
        assert_ne!(a, b);
        let mut chars: Vec<char> = a.chars().collect();
        let i = chars.len() - 1;
        chars[i] = if chars[i] == 'A' { 'B' } else { 'A' };
        let bad: String = chars.into_iter().collect();
        assert!(decrypt("p", &bad).is_err());
    }
}
