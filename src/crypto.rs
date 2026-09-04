//! 密文格式：`MX1.` + base64(iv[12] || ciphertext || tag[16])
//! AES-256-GCM，密码经 sha256 摊成 32 字节密钥（任意长度都行）。

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::RngCore;
use rsa::pkcs8::{
    DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding,
};
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PREFIX: &str = "MX1.";
pub const IDENTITY_PREFIX: &str = "MXID1.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMeta {
    pub name: String,
    pub extension: String,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub kind: String,
    pub meta: Option<FileMeta>,
    pub data: String,
}

pub fn encrypt_payload(password: &str, bytes: &[u8]) -> Result<String, String> {
    let cipher = cipher_from_password(password);
    let mut iv = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut iv);
    let ct = cipher
        .encrypt(Nonce::from_slice(&iv), bytes)
        .map_err(|_| "加密失败".to_string())?;
    let mut blob = Vec::with_capacity(12 + ct.len());
    blob.extend_from_slice(&iv);
    blob.extend_from_slice(&ct);
    Ok(format!("{PREFIX}{}", STANDARD.encode(blob)))
}

pub fn decrypt_payload(password: &str, token: &str) -> Result<Vec<u8>, String> {
    let compact: String = token.chars().filter(|c| !c.is_whitespace()).collect();
    let body = compact.strip_prefix(PREFIX).unwrap_or(&compact);
    let blob = STANDARD
        .decode(body)
        .map_err(|_| "这不是可识别的密信内容。".to_string())?;
    if blob.len() < 28 {
        return Err("内容不完整，无法解密。".into());
    }
    cipher_from_password(password)
        .decrypt(Nonce::from_slice(&blob[..12]), &blob[12..])
        .map_err(|_| "口令不对，或内容已被改动。".into())
}

#[allow(dead_code)]
pub fn encrypt(password: &str, plain: &str) -> Result<String, String> {
    encrypt_payload(password, plain.as_bytes())
}

#[allow(dead_code)]
pub fn decrypt(password: &str, token: &str) -> Result<String, String> {
    String::from_utf8(decrypt_payload(password, token)?).map_err(|_| "解密出来不是文字。".into())
}

pub fn generate_identity() -> Result<(String, String), String> {
    let mut rng = rand::thread_rng();
    let private = RsaPrivateKey::new(&mut rng, 2048).map_err(|e| e.to_string())?;
    let public = RsaPublicKey::from(&private);
    Ok((
        private
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| e.to_string())?
            .to_string(),
        public
            .to_public_key_pem(LineEnding::LF)
            .map_err(|e| e.to_string())?,
    ))
}

pub fn public_from_private(private_pem: &str) -> Result<String, String> {
    let private =
        RsaPrivateKey::from_pkcs8_pem(private_pem).map_err(|_| "身份文件无效。".to_string())?;
    RsaPublicKey::from(&private)
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| e.to_string())
}

pub fn encrypt_for(public_pem: &str, bytes: &[u8]) -> Result<String, String> {
    let public = RsaPublicKey::from_public_key_pem(public_pem)
        .map_err(|_| "对方身份信息无法识别。".to_string())?;
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    let mut iv = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut iv);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let ct = cipher
        .encrypt(Nonce::from_slice(&iv), bytes)
        .map_err(|_| "加密失败".to_string())?;
    let wrapped = public
        .encrypt(&mut rand::thread_rng(), Oaep::new::<Sha256>(), &key)
        .map_err(|_| "对方身份不支持加密。".to_string())?;
    let mut blob = Vec::new();
    blob.extend_from_slice(&wrapped);
    blob.extend_from_slice(&iv);
    blob.extend_from_slice(&ct);
    Ok(format!("{IDENTITY_PREFIX}{}", STANDARD.encode(blob)))
}

pub fn decrypt_with(private_pem: &str, token: &str) -> Result<Vec<u8>, String> {
    let private =
        RsaPrivateKey::from_pkcs8_pem(private_pem).map_err(|_| "本机身份文件无效。".to_string())?;
    let compact: String = token.chars().filter(|c| !c.is_whitespace()).collect();
    let body = compact
        .strip_prefix(IDENTITY_PREFIX)
        .ok_or("这不是身份加密内容。")?;
    let blob = STANDARD
        .decode(body)
        .map_err(|_| "内容不完整，无法解密。".to_string())?;
    if blob.len() < 284 {
        return Err("内容不完整，无法解密。".into());
    }
    let key = private
        .decrypt(Oaep::new::<Sha256>(), &blob[..256])
        .map_err(|_| "这份内容不是发给本机身份的。".to_string())?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    cipher
        .decrypt(Nonce::from_slice(&blob[256..268]), &blob[268..])
        .map_err(|_| "解密失败，内容可能已被改动。".into())
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
