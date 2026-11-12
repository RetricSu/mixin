//! 密文格式：
//! - 口令密信：`MX1.` + base64(salt[16] || iv[12] || ciphertext || tag[16])
//!   AES-256-GCM，口令经 Argon2id（m=19456 KiB, t=2, p=1）与随机 16 字节 salt 派生 32 字节密钥。
//! - 专属密信：`MXID2.` + base64(sender_public_key[64] || ephemeral_public_key[32] || iv[12] || signature[64] || ciphertext || tag[16])
//!   基于 X25519 临时密钥交换派生会话密钥，AES-256-GCM 认证加密，发件人经 Ed25519 数字签名防伪与防篡改。

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

pub const PREFIX: &str = "MX1.";
pub const IDENTITY_PREFIX: &str = "MXID2.";

pub const PEM_PRIVATE_HEADER: &str = "-----BEGIN WAXSEAL PRIVATE IDENTITY-----";
pub const PEM_PRIVATE_FOOTER: &str = "-----END WAXSEAL PRIVATE IDENTITY-----";
pub const PEM_PUBLIC_HEADER: &str = "-----BEGIN WAXSEAL PUBLIC KEY-----";
pub const PEM_PUBLIC_FOOTER: &str = "-----END WAXSEAL PUBLIC KEY-----";

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

/// 专属密文固定头部长度：
/// sender_public_key (64 bytes: Ed25519 32B + X25519 32B) +
/// ephemeral_x25519_pk (32 bytes) +
/// iv (12 bytes) +
/// signature (64 bytes) = 172 字节
pub const IDENTITY_HEADER_LEN: usize = 64 + 32 + NONCE_LEN + 64;
/// 专属密文最小有效负载长度（Header + Tag = 188 字节）
pub const MIN_IDENTITY_PAYLOAD_LEN: usize = IDENTITY_HEADER_LEN + TAG_LEN;

// 保留历史域分离标签，确保存量密文（MXID2.）及本地身份密钥向后兼容
const SIGN_DOMAIN_TAG: &[u8] = b"mixin-x25519-ed25519-v1";
const HKDF_AES_INFO: &[u8] = b"mixin-x25519-aes-gcm-key";

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

// ---------------------------------------------------------------------------
// 身份与密钥体系 (X25519 + Ed25519)
// ---------------------------------------------------------------------------

fn derive_keys_from_seed(seed: &[u8; 32]) -> (SigningKey, StaticSecret) {
    let hkdf = Hkdf::<Sha256>::new(None, seed);
    let mut ed_seed = [0u8; 32];
    hkdf.expand(b"mixin-ed25519-seed", &mut ed_seed)
        .expect("HKDF expand for Ed25519 seed must succeed");
    let signing_key = SigningKey::from_bytes(&ed_seed);

    let mut x_seed = [0u8; 32];
    hkdf.expand(b"mixin-x25519-seed", &mut x_seed)
        .expect("HKDF expand for X25519 seed must succeed");
    let static_secret = StaticSecret::from(x_seed);

    (signing_key, static_secret)
}

fn parse_private_identity(private_pem: &str) -> Result<(SigningKey, StaticSecret), String> {
    let raw: String = private_pem
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("-----"))
        .collect();
    let bytes = STANDARD
        .decode(&raw)
        .map_err(|_| "身份私钥格式损坏。".to_string())?;

    if bytes.len() == 32 {
        let seed: [u8; 32] = bytes.try_into().unwrap();
        Ok(derive_keys_from_seed(&seed))
    } else if bytes.len() == 64 {
        let ed_seed: [u8; 32] = bytes[0..32].try_into().unwrap();
        let x_seed: [u8; 32] = bytes[32..64].try_into().unwrap();
        Ok((SigningKey::from_bytes(&ed_seed), StaticSecret::from(x_seed)))
    } else {
        Err("身份私钥长度不正确。".to_string())
    }
}

pub fn parse_public_key(public_str: &str) -> Result<(VerifyingKey, PublicKey), String> {
    let raw: String = public_str
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("-----"))
        .collect();
    let clean = raw
        .strip_prefix("WSPK1.")
        .or_else(|| raw.strip_prefix("MXPK1."))
        .unwrap_or(&raw);
    let bytes = STANDARD
        .decode(clean)
        .map_err(|_| "无效的公开身份代码，请确认复制完整。".to_string())?;

    if bytes.len() != 64 {
        return Err("公开身份数据长度无效。".to_string());
    }

    let ed_bytes: [u8; 32] = bytes[0..32].try_into().unwrap();
    let x_bytes: [u8; 32] = bytes[32..64].try_into().unwrap();

    let verifying_key = VerifyingKey::from_bytes(&ed_bytes)
        .map_err(|_| "公开身份签名公钥无效。".to_string())?;
    let public_key = PublicKey::from(x_bytes);

    Ok((verifying_key, public_key))
}

fn format_public_identity(ed_pk: &VerifyingKey, x_pk: &PublicKey) -> String {
    let mut blob = Vec::with_capacity(64);
    blob.extend_from_slice(ed_pk.as_bytes());
    blob.extend_from_slice(x_pk.as_bytes());
    format!(
        "{PEM_PUBLIC_HEADER}\n{}\n{PEM_PUBLIC_FOOTER}",
        STANDARD.encode(blob)
    )
}

pub fn generate_identity() -> Result<(String, String), String> {
    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);
    let (signing_key, static_secret) = derive_keys_from_seed(&seed);
    let ed_pk = signing_key.verifying_key();
    let x_pk = PublicKey::from(&static_secret);

    let private_pem = format!(
        "{PEM_PRIVATE_HEADER}\n{}\n{PEM_PRIVATE_FOOTER}",
        STANDARD.encode(seed)
    );
    let public_pem = format_public_identity(&ed_pk, &x_pk);

    Ok((private_pem, public_pem))
}

pub fn public_from_private(private_pem: &str) -> Result<String, String> {
    let (signing_key, static_secret) = parse_private_identity(private_pem)?;
    let ed_pk = signing_key.verifying_key();
    let x_pk = PublicKey::from(&static_secret);
    Ok(format_public_identity(&ed_pk, &x_pk))
}

pub fn compute_fingerprint(public_str: &str) -> String {
    let hash = if let Ok((ed_pk, x_pk)) = parse_public_key(public_str) {
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(ed_pk.as_bytes());
        buf[32..].copy_from_slice(x_pk.as_bytes());
        Sha256::digest(buf)
    } else {
        let clean: String = public_str.chars().filter(|c| !c.is_whitespace()).collect();
        Sha256::digest(clean.as_bytes())
    };
    format!(
        "{:02X}{:02X}·{:02X}{:02X}·{:02X}{:02X}·{:02X}{:02X}",
        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]
    )
}

pub fn compute_fingerprint_from_bytes(bytes64: &[u8; 64]) -> String {
    let hash = Sha256::digest(bytes64);
    format!(
        "{:02X}{:02X}·{:02X}{:02X}·{:02X}{:02X}·{:02X}{:02X}",
        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]
    )
}

pub fn format_fingerprint_spaced(fp: &str) -> String {
    fp.replace(" · ", "·").replace('·', " · ")
}

pub fn validate_public_key(public_str: &str) -> Result<String, String> {
    parse_public_key(public_str).map(|(ed_pk, x_pk)| {
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(ed_pk.as_bytes());
        buf[32..].copy_from_slice(x_pk.as_bytes());
        compute_fingerprint_from_bytes(&buf)
    })
}

// ---------------------------------------------------------------------------
// 专属密信加解密 (X25519 交换 + Ed25519 签名 + AES-256-GCM)
// ---------------------------------------------------------------------------

pub fn encrypt_for(
    sender_private: &str,
    recipient_public: &str,
    bytes: &[u8],
) -> Result<String, String> {
    let (sender_signing_key, sender_x_secret) = parse_private_identity(sender_private)?;
    let sender_ed_pk = sender_signing_key.verifying_key();
    let sender_x_pk = PublicKey::from(&sender_x_secret);

    let mut sender_pk_bytes = [0u8; 64];
    sender_pk_bytes[..32].copy_from_slice(sender_ed_pk.as_bytes());
    sender_pk_bytes[32..].copy_from_slice(sender_x_pk.as_bytes());

    let (_recipient_ed_pk, recipient_x_pk) = parse_public_key(recipient_public)?;

    // 1. 生成临时 X25519 密钥对并计算共享密钥
    let mut rng = rand::thread_rng();
    let ephemeral_secret = StaticSecret::random_from_rng(&mut rng);
    let ephemeral_public = PublicKey::from(&ephemeral_secret);
    let shared_secret = ephemeral_secret.diffie_hellman(&recipient_x_pk);

    // 2. 通过 HKDF-SHA256 派生 32 字节 AES-256 密钥
    let hkdf = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
    let mut aes_key = [0u8; KEY_LEN];
    hkdf.expand(HKDF_AES_INFO, &mut aes_key)
        .map_err(|e| format!("密钥派生错误: {e}"))?;

    // 3. 随机 12 字节 IV 并进行 AES-256-GCM 加密
    let mut iv = [0u8; NONCE_LEN];
    rng.fill_bytes(&mut iv);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&aes_key));
    let ct = cipher
        .encrypt(Nonce::from_slice(&iv), bytes)
        .map_err(|_| "加密失败".to_string())?;

    // 4. Ed25519 发件人签名（绑定协议域标签、接收方公钥、发件人公钥、临时公钥、IV 及密文）
    let mut sign_data = Vec::with_capacity(
        SIGN_DOMAIN_TAG.len() + 32 + 64 + 32 + NONCE_LEN + ct.len(),
    );
    sign_data.extend_from_slice(SIGN_DOMAIN_TAG);
    sign_data.extend_from_slice(recipient_x_pk.as_bytes());
    sign_data.extend_from_slice(&sender_pk_bytes);
    sign_data.extend_from_slice(ephemeral_public.as_bytes());
    sign_data.extend_from_slice(&iv);
    sign_data.extend_from_slice(&ct);
    let signature = sender_signing_key.sign(&sign_data);

    // 5. 打包密文：sender_pk[64] || ephemeral_pk[32] || iv[12] || signature[64] || ct
    let mut blob = Vec::with_capacity(IDENTITY_HEADER_LEN + ct.len());
    blob.extend_from_slice(&sender_pk_bytes);
    blob.extend_from_slice(ephemeral_public.as_bytes());
    blob.extend_from_slice(&iv);
    blob.extend_from_slice(&signature.to_bytes());
    blob.extend_from_slice(&ct);

    Ok(format!("{IDENTITY_PREFIX}{}", STANDARD.encode(blob)))
}

pub fn decrypt_with_sender(
    recipient_private: &str,
    token: &str,
) -> Result<(Vec<u8>, String), String> {
    let (_recipient_signing_key, recipient_x_secret) = parse_private_identity(recipient_private)?;
    let recipient_x_pk = PublicKey::from(&recipient_x_secret);

    let compact: String = token.chars().filter(|c| !c.is_whitespace()).collect();
    let body = compact
        .strip_prefix(IDENTITY_PREFIX)
        .ok_or_else(|| "这不是专属身份加密内容。".to_string())?;
    let blob = STANDARD
        .decode(body)
        .map_err(|_| "密信内容格式损坏，无法解密。".to_string())?;

    if blob.len() < MIN_IDENTITY_PAYLOAD_LEN {
        return Err("内容不完整，无法解密。".into());
    }

    let sender_pk_bytes: [u8; 64] = blob[0..64].try_into().unwrap();
    let ephemeral_pk_bytes: [u8; 32] = blob[64..96].try_into().unwrap();
    let iv = &blob[96..96 + NONCE_LEN];
    let signature_bytes: [u8; 64] = blob[96 + NONCE_LEN..IDENTITY_HEADER_LEN].try_into().unwrap();
    let ct = &blob[IDENTITY_HEADER_LEN..];

    // 1. 验证发件人公钥有效性与数字签名
    let sender_ed_pk_bytes: [u8; 32] = sender_pk_bytes[0..32].try_into().unwrap();
    let sender_ed_vk = VerifyingKey::from_bytes(&sender_ed_pk_bytes)
        .map_err(|_| "发件人身份公钥无效。".to_string())?;
    let signature = Signature::from_bytes(&signature_bytes);

    let mut sign_data = Vec::with_capacity(
        SIGN_DOMAIN_TAG.len() + 32 + 64 + 32 + NONCE_LEN + ct.len(),
    );
    sign_data.extend_from_slice(SIGN_DOMAIN_TAG);
    sign_data.extend_from_slice(recipient_x_pk.as_bytes());
    sign_data.extend_from_slice(&sender_pk_bytes);
    sign_data.extend_from_slice(&ephemeral_pk_bytes);
    sign_data.extend_from_slice(iv);
    sign_data.extend_from_slice(ct);

    sender_ed_vk
        .verify(&sign_data, &signature)
        .map_err(|_| "发件人签名验证失败，内容可能已被篡改。".to_string())?;

    // 2. Diffie-Hellman 密钥交换与 HKDF 对称密钥派生
    let ephemeral_x_pk = PublicKey::from(ephemeral_pk_bytes);
    let shared_secret = recipient_x_secret.diffie_hellman(&ephemeral_x_pk);

    let hkdf = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
    let mut aes_key = [0u8; KEY_LEN];
    hkdf.expand(HKDF_AES_INFO, &mut aes_key)
        .map_err(|e| format!("密钥派生错误: {e}"))?;

    // 3. AES-256-GCM 解密
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&aes_key));
    let plain = cipher
        .decrypt(Nonce::from_slice(iv), ct)
        .map_err(|_| "解密失败，内容可能非发给本机身份或已被损坏。".to_string())?;

    let sender_fingerprint = compute_fingerprint_from_bytes(&sender_pk_bytes);
    Ok((plain, sender_fingerprint))
}

#[allow(dead_code)]
pub fn decrypt_with(recipient_private: &str, token: &str) -> Result<Vec<u8>, String> {
    decrypt_with_sender(recipient_private, token).map(|(plain, _)| plain)
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
        let short_blob = vec![0u8; 43];
        let short_token = format!("{PREFIX}{}", STANDARD.encode(&short_blob));
        let err = decrypt("p", &short_token).unwrap_err();
        assert!(err.contains("内容不完整，无法解密。"));

        let bad_token = format!("{PREFIX}@@@invalid-base64!!!");
        let err = decrypt("p", &bad_token).unwrap_err();
        assert!(err.contains("这不是可识别的密信内容。"));
    }

    #[test]
    fn helpers_test() {
        let (_private, public) = generate_identity().unwrap();
        let fp = compute_fingerprint(&public);
        assert_eq!(fp.chars().count(), 19);
        assert!(validate_public_key(&public).is_ok());
        assert!(validate_public_key("invalid key").is_err());

        let pwd = generate_memorable_password();
        assert!(pwd.contains('-'));

        assert_eq!(format_fingerprint_spaced("A1B2·C3D4·E5F6·7890"), "A1B2 · C3D4 · E5F6 · 7890");
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(2048), "2.0 KB");
        assert_eq!(format_file_size(2 * 1024 * 1024), "2.0 MB");
    }

    #[test]
    fn test_x25519_ed25519_identity_and_crypto() {
        let (alice_priv, alice_pub) = generate_identity().unwrap();
        let (bob_priv, bob_pub) = generate_identity().unwrap();
        let (charlie_priv, _charlie_pub) = generate_identity().unwrap();

        assert_eq!(public_from_private(&alice_priv).unwrap(), alice_pub);
        assert_eq!(public_from_private(&bob_priv).unwrap(), bob_pub);

        let alice_fp = compute_fingerprint(&alice_pub);
        let bob_fp = compute_fingerprint(&bob_pub);
        assert_eq!(validate_public_key(&alice_pub).unwrap(), alice_fp);
        assert_eq!(validate_public_key(&bob_pub).unwrap(), bob_fp);

        // Alice encrypts for Bob
        let message = b"Confidential memo: meeting at dawn.";
        let token = encrypt_for(&alice_priv, &bob_pub, message).unwrap();
        assert!(token.starts_with(IDENTITY_PREFIX));

        // Ephemeral forward secrecy: encrypting the same message again produces different token
        let token2 = encrypt_for(&alice_priv, &bob_pub, message).unwrap();
        assert_ne!(token, token2, "Fresh ephemeral keys must produce distinct ciphertexts");

        // Bob decrypts and verifies sender
        let (plain, sender_fp) = decrypt_with_sender(&bob_priv, &token).unwrap();
        assert_eq!(&plain, message);
        assert_eq!(sender_fp, alice_fp);

        // Standard decrypt_with also works
        assert_eq!(decrypt_with(&bob_priv, &token).unwrap(), message);

        // Alice (sender) cannot decrypt Bob's message
        assert!(decrypt_with(&alice_priv, &token).is_err());

        // Charlie (third party) cannot decrypt Bob's message
        assert!(decrypt_with(&charlie_priv, &token).is_err());

        // Forwarding attack: attempting to replay Alice's message intended for Bob to Charlie must fail
        assert!(decrypt_with(&charlie_priv, &token).is_err());

        // Decode payload for tampering tests
        let body = token.strip_prefix(IDENTITY_PREFIX).unwrap();
        let raw_blob = STANDARD.decode(body).unwrap();
        assert!(raw_blob.len() >= MIN_IDENTITY_PAYLOAD_LEN);

        // 1. Tamper with signature
        let mut tampered_sig = raw_blob.clone();
        tampered_sig[110] ^= 0xFF; // within signature range [108..172]
        let tampered_sig_token = format!("{IDENTITY_PREFIX}{}", STANDARD.encode(&tampered_sig));
        let err = decrypt_with(&bob_priv, &tampered_sig_token).unwrap_err();
        assert!(err.contains("签名验证失败"));

        // 2. Tamper with ephemeral public key
        let mut tampered_eph = raw_blob.clone();
        tampered_eph[70] ^= 0xFF; // within ephemeral pk range [64..96]
        let tampered_eph_token = format!("{IDENTITY_PREFIX}{}", STANDARD.encode(&tampered_eph));
        let err = decrypt_with(&bob_priv, &tampered_eph_token).unwrap_err();
        assert!(err.contains("签名验证失败") || err.contains("解密失败"));

        // 3. Tamper with IV
        let mut tampered_iv = raw_blob.clone();
        tampered_iv[100] ^= 0xFF; // within IV range [96..108]
        let tampered_iv_token = format!("{IDENTITY_PREFIX}{}", STANDARD.encode(&tampered_iv));
        let err = decrypt_with(&bob_priv, &tampered_iv_token).unwrap_err();
        assert!(err.contains("签名验证失败"));

        // 4. Tamper with sender public key
        let mut tampered_sender = raw_blob.clone();
        tampered_sender[10] ^= 0xFF; // within sender pk range [0..64]
        let tampered_sender_token = format!("{IDENTITY_PREFIX}{}", STANDARD.encode(&tampered_sender));
        let err = decrypt_with(&bob_priv, &tampered_sender_token).unwrap_err();
        assert!(err.contains("签名验证失败") || err.contains("公钥无效"));

        // 5. Tamper with ciphertext / tag
        let mut tampered_ct = raw_blob.clone();
        let last = tampered_ct.len() - 1;
        tampered_ct[last] ^= 0xFF;
        let tampered_ct_token = format!("{IDENTITY_PREFIX}{}", STANDARD.encode(&tampered_ct));
        let err = decrypt_with(&bob_priv, &tampered_ct_token).unwrap_err();
        assert!(err.contains("签名验证失败") || err.contains("解密失败"));

        // 6. Truncated identity payload (less than 188 bytes)
        let truncated = vec![0u8; MIN_IDENTITY_PAYLOAD_LEN - 1];
        let trunc_token = format!("{IDENTITY_PREFIX}{}", STANDARD.encode(&truncated));
        let err = decrypt_with(&bob_priv, &trunc_token).unwrap_err();
        assert!(err.contains("内容不完整，无法解密。"));

        // 7. Non-identity token passed to decrypt_with
        let non_id_token = format!("{PREFIX}AQIDBA==");
        let err = decrypt_with(&bob_priv, &non_id_token).unwrap_err();
        assert!(err.contains("这不是专属身份加密内容"));
    }

    #[test]
    fn legacy_mixin_pem_headers_remain_compatible() {
        let (priv_pem, pub_pem) = generate_identity().unwrap();
        assert!(priv_pem.starts_with(PEM_PRIVATE_HEADER));
        assert!(pub_pem.starts_with(PEM_PUBLIC_HEADER));

        let legacy_priv = priv_pem
            .replace(PEM_PRIVATE_HEADER, "-----BEGIN MIXIN PRIVATE IDENTITY-----")
            .replace(PEM_PRIVATE_FOOTER, "-----END MIXIN PRIVATE IDENTITY-----");
        let legacy_pub = pub_pem
            .replace(PEM_PUBLIC_HEADER, "-----BEGIN MIXIN PUBLIC KEY-----")
            .replace(PEM_PUBLIC_FOOTER, "-----END MIXIN PUBLIC KEY-----");

        assert_eq!(public_from_private(&legacy_priv).unwrap(), pub_pem);
        assert_eq!(
            compute_fingerprint(&legacy_pub),
            compute_fingerprint(&pub_pem)
        );

        let token = encrypt_for(&legacy_priv, &legacy_pub, b"legacy-compat").unwrap();
        assert_eq!(decrypt_with(&priv_pem, &token).unwrap(), b"legacy-compat");
    }
}
