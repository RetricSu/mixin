//! 密文格式：`MX1.` + base64(salt[16] || iv[12] || ciphertext || tag[16])
//! AES-256-GCM，口令经 Argon2id（m=19456 KiB, t=2, p=1）与随机 16 字节 salt 派生 32 字节密钥。

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
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

/// Argon2id 内存开销（KiB），符合 OWASP / RFC 9106 推荐配置（19 MiB）
pub const ARGON2_M_COST: u32 = 19456;
/// Argon2id 迭代轮数
pub const ARGON2_T_COST: u32 = 2;
/// Argon2id 并行度（并发线程数）
pub const ARGON2_P_COST: u32 = 1;
/// 派生对称密钥长度（AES-256 需要 32 字节）
pub const KEY_LEN: usize = 32;
/// 随机 salt 长度（16 字节 / 128 位）
pub const SALT_LEN: usize = 16;
/// AES-GCM Nonce / IV 长度（12 字节 / 96 位）
pub const NONCE_LEN: usize = 12;
/// AES-GCM 认证 Tag 长度（16 字节 / 128 位）
pub const TAG_LEN: usize = 16;
/// 口令密文最小有效负载长度（Salt + Nonce + Tag = 44 字节）
pub const MIN_PASSWORD_PAYLOAD_LEN: usize = SALT_LEN + NONCE_LEN + TAG_LEN;

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

pub fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], String> {
    let params = Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(KEY_LEN))
        .map_err(|e| format!("Argon2 参数错误: {e}"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("口令密钥派生失败: {e}"))?;
    Ok(key)
}

fn cipher_from_password(password: &str, salt: &[u8]) -> Result<Aes256Gcm, String> {
    let mut key = derive_key(password, salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    key.fill(0);
    Ok(cipher)
}

pub fn encrypt_payload(password: &str, bytes: &[u8]) -> Result<String, String> {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    let cipher = cipher_from_password(password, &salt)?;
    let mut iv = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut iv);
    let ct = cipher
        .encrypt(Nonce::from_slice(&iv), bytes)
        .map_err(|_| "加密失败".to_string())?;
    let mut blob = Vec::with_capacity(SALT_LEN + NONCE_LEN + ct.len());
    blob.extend_from_slice(&salt);
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
    if blob.len() < MIN_PASSWORD_PAYLOAD_LEN {
        return Err("内容不完整，无法解密。".into());
    }
    let salt = &blob[..SALT_LEN];
    let iv = &blob[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ct = &blob[SALT_LEN + NONCE_LEN..];
    let cipher = cipher_from_password(password, salt)?;
    cipher
        .decrypt(Nonce::from_slice(iv), ct)
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

pub fn compute_fingerprint(public_pem: &str) -> String {
    let clean: String = public_pem.chars().filter(|c| !c.is_whitespace()).collect();
    let hash = Sha256::digest(clean.as_bytes());
    format!(
        "{:02X}{:02X}·{:02X}{:02X}·{:02X}{:02X}·{:02X}{:02X}",
        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]
    )
}

pub fn format_fingerprint_spaced(fp: &str) -> String {
    fp.replace(" · ", "·").replace('·', " · ")
}

pub fn validate_public_key(pem: &str) -> Result<String, String> {
    RsaPublicKey::from_public_key_pem(pem)
        .map(|_| compute_fingerprint(pem))
        .map_err(|_| "无效的公开身份，请确保包含完整的 -----BEGIN PUBLIC KEY-----".to_string())
}

pub fn format_file_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn generate_memorable_password() -> String {
    const ADJECTIVES: &[&str] = &[
        "swift", "brave", "calm", "amber", "silver", "golden", "silent", "clever", "gentle",
        "cosmic", "bright", "quiet", "noble", "frosty", "lucky", "solar", "polar", "shadow",
        "winter", "summer",
    ];
    const NOUNS: &[&str] = &[
        "river", "forest", "mountain", "falcon", "haven", "breeze", "beacon", "valley", "shield",
        "island", "meadow", "harbor", "zenith", "castle", "bridge", "garden", "planet", "timber",
        "stream", "canyon",
    ];
    let mut rng = rand::thread_rng();
    use rand::seq::SliceRandom;
    use rand::Rng;
    let adj = ADJECTIVES.choose(&mut rng).unwrap_or(&"safe");
    let noun = NOUNS.choose(&mut rng).unwrap_or(&"box");
    let num: u32 = rng.gen_range(1000..9999);
    format!("{adj}-{noun}-{num}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2id_key_derivation_properties() {
        let salt1 = [1u8; SALT_LEN];
        let salt2 = [2u8; SALT_LEN];

        let k1 = derive_key("password123", &salt1).unwrap();
        let k2 = derive_key("password123", &salt1).unwrap();
        assert_eq!(k1, k2, "Same password and same salt must yield identical key");

        let k3 = derive_key("password123", &salt2).unwrap();
        assert_ne!(k1, k3, "Different salts must yield different keys");

        let k4 = derive_key("different_pwd", &salt1).unwrap();
        assert_ne!(k1, k4, "Different passwords must yield different keys");

        assert_eq!(k1.len(), 32, "Argon2id derived key must be 32 bytes for AES-256");
    }

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
    fn random_salt_and_iv_and_tamper() {
        let a = encrypt("p", "same content").unwrap();
        let b = encrypt("p", "same content").unwrap();
        assert_ne!(a, b);

        let body_a = a.strip_prefix(PREFIX).unwrap();
        let body_b = b.strip_prefix(PREFIX).unwrap();
        let blob_a = STANDARD.decode(body_a).unwrap();
        let blob_b = STANDARD.decode(body_b).unwrap();

        // 16 bytes salt + 12 bytes nonce + ciphertext
        assert!(blob_a.len() >= MIN_PASSWORD_PAYLOAD_LEN);
        assert!(blob_b.len() >= MIN_PASSWORD_PAYLOAD_LEN);

        let salt_a = &blob_a[..SALT_LEN];
        let salt_b = &blob_b[..SALT_LEN];
        assert_ne!(salt_a, salt_b, "Each encryption must use a fresh random salt");

        let iv_a = &blob_a[SALT_LEN..SALT_LEN + NONCE_LEN];
        let iv_b = &blob_b[SALT_LEN..SALT_LEN + NONCE_LEN];
        assert_ne!(iv_a, iv_b, "Each encryption must use a fresh random IV");

        // Tampering with salt
        let mut tampered_salt_blob = blob_a.clone();
        tampered_salt_blob[0] ^= 0xFF;
        let tampered_salt_token = format!("{PREFIX}{}", STANDARD.encode(&tampered_salt_blob));
        assert!(decrypt("p", &tampered_salt_token).is_err());

        // Tampering with IV
        let mut tampered_iv_blob = blob_a.clone();
        tampered_iv_blob[SALT_LEN] ^= 0xFF;
        let tampered_iv_token = format!("{PREFIX}{}", STANDARD.encode(&tampered_iv_blob));
        assert!(decrypt("p", &tampered_iv_token).is_err());

        // Tampering with ciphertext / tag
        let mut tampered_ct_blob = blob_a.clone();
        let last_idx = tampered_ct_blob.len() - 1;
        tampered_ct_blob[last_idx] ^= 0xFF;
        let tampered_ct_token = format!("{PREFIX}{}", STANDARD.encode(&tampered_ct_blob));
        assert!(decrypt("p", &tampered_ct_token).is_err());
    }

    #[test]
    fn malformed_and_truncated_payload() {
        // Shorter than 44 bytes (MIN_PASSWORD_PAYLOAD_LEN)
        let short_blob = vec![0u8; 43];
        let short_token = format!("{PREFIX}{}", STANDARD.encode(&short_blob));
        let err = decrypt("p", &short_token).unwrap_err();
        assert!(err.contains("内容不完整，无法解密。"));

        // Invalid base64
        let bad_token = format!("{PREFIX}@@@invalid-base64!!!");
        let err = decrypt("p", &bad_token).unwrap_err();
        assert!(err.contains("这不是可识别的密信内容。"));
    }

    #[test]
    fn helpers_test() {
        let (_private, public) = generate_identity().unwrap();
        let fp = compute_fingerprint(&public);
        assert_eq!(fp.chars().count(), 19); // 4 groups of 4 hex chars separated by 3 dots
        assert!(validate_public_key(&public).is_ok());
        assert!(validate_public_key("invalid key").is_err());

        let pwd = generate_memorable_password();
        assert!(pwd.contains('-'));

        assert_eq!(format_fingerprint_spaced("A1B2·C3D4·E5F6·7890"), "A1B2 · C3D4 · E5F6 · 7890");
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(2048), "2.0 KB");
        assert_eq!(format_file_size(2 * 1024 * 1024), "2.0 MB");
    }
}
