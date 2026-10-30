//! 密信桌面版：面向普通人的极简点对点安全传递工具
mod contacts;
mod crypto;
mod theme;
mod ui;

use base64::Engine as _;
use contacts::{add_contact, load_contacts, remove_contact, Contact};
use crypto::{
    compute_fingerprint, decrypt_payload, decrypt_with, encrypt_for, encrypt_payload,
    format_file_size, generate_identity, generate_memorable_password, public_from_private,
    validate_public_key, Envelope, FileMeta, IDENTITY_PREFIX, PREFIX,
};
use eframe::egui::{self, Color32};
use std::{fs, path::PathBuf, time::Instant};
use theme::ThemeColors;

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Send,
    Receive,
    Identity,
}

#[derive(PartialEq, Clone, Copy)]
enum SendMode {
    Password,
    Contact,
}

#[derive(PartialEq, Clone, Copy)]
enum PayloadType {
    Text,
    File,
}

#[derive(Clone)]
enum ToastKind {
    Success,
    Info,
    Warning,
    Error,
}

#[derive(Clone)]
struct Toast {
    message: String,
    kind: ToastKind,
    created_at: Instant,
    duration_secs: f32,
}

#[derive(Clone)]
struct SelectedFile {
    #[allow(dead_code)]
    path: PathBuf,
    name: String,
    extension: String,
    size: usize,
    bytes: Vec<u8>,
}

#[derive(Clone)]
enum DecryptedContent {
    Text(String),
    File {
        meta: FileMeta,
        bytes: Vec<u8>,
        saved_path: Option<PathBuf>,
    },
}

struct App {
    tab: Tab,
    dark_mode: bool,
    toast: Option<Toast>,

    // --- 发送状态 ---
    send_mode: SendMode,
    payload_type: PayloadType,
    send_text: String,
    send_file: Option<SelectedFile>,
    send_password: String,
    send_confirm: String,
    send_show_password: bool,
    selected_contact_idx: Option<usize>,
    recipient_custom_key: String,
    generated_cipher: String,
    generated_summary: Option<String>,

    // --- 接收状态 ---
    receive_input: String,
    receive_password: String,
    receive_show_password: bool,
    decrypted: Option<DecryptedContent>,

    // --- 身份与联系人状态 ---
    identity: (String, String), // (private_pem, public_pem)
    my_fingerprint: String,
    show_regenerate_confirm: bool,
    contacts: Vec<Contact>,
    new_contact_name: String,
    new_contact_key: String,
    show_add_contact_dialog: bool,
    focus_new_contact: bool,
    confirm_delete_idx: Option<usize>,
}

impl App {
    fn new() -> Self {
        let identity =
            load_identity().unwrap_or_else(|| generate_identity().expect("无法生成本机身份"));
        save_identity(&identity.0);
        Self::with_identity(identity, load_contacts(), load_theme_preference())
    }

    fn with_identity(identity: (String, String), contacts: Vec<Contact>, dark_mode: bool) -> Self {
        let my_fingerprint = compute_fingerprint(&identity.1);
        Self {
            tab: Tab::Send,
            dark_mode,
            toast: None,

            send_mode: SendMode::Password,
            payload_type: PayloadType::Text,
            send_text: String::new(),
            send_file: None,
            send_password: String::new(),
            send_confirm: String::new(),
            send_show_password: false,
            selected_contact_idx: None,
            recipient_custom_key: String::new(),
            generated_cipher: String::new(),
            generated_summary: None,

            receive_input: String::new(),
            receive_password: String::new(),
            receive_show_password: false,
            decrypted: None,

            identity,
            my_fingerprint,
            show_regenerate_confirm: false,
            contacts,
            new_contact_name: String::new(),
            new_contact_key: String::new(),
            show_add_contact_dialog: false,
            focus_new_contact: false,
            confirm_delete_idx: None,
        }
    }

    fn theme(&self) -> ThemeColors {
        if self.dark_mode {
            theme::DARK_THEME
        } else {
            theme::LIGHT_THEME
        }
    }

    fn notify(&mut self, message: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast {
            message: message.into(),
            duration_secs: if matches!(kind, ToastKind::Error | ToastKind::Warning) {
                10.0
            } else {
                3.5
            },
            kind,
            created_at: Instant::now(),
        });
    }

    fn copy_text(&mut self, ctx: &egui::Context, text: String, feedback: &str) {
        ctx.copy_text(text.clone());
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(text);
        }
        self.notify(feedback, ToastKind::Success);
    }

    fn load_file_for_send(&mut self, path: PathBuf) {
        match fs::read(&path) {
            Ok(bytes) => {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let extension = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let size = bytes.len();
                self.send_file = Some(SelectedFile {
                    path,
                    name,
                    extension,
                    size,
                    bytes,
                });
                self.payload_type = PayloadType::File;
                self.generated_cipher.clear();
                self.generated_summary = None;
                self.notify("文件载入成功", ToastKind::Success);
            }
            Err(e) => {
                self.notify(format!("读取文件失败：{e}"), ToastKind::Error);
            }
        }
    }

    fn send_blocker(&self) -> Option<&'static str> {
        if (self.payload_type == PayloadType::File && self.send_file.is_none())
            || (self.payload_type == PayloadType::Text && self.send_text.trim().is_empty())
        {
            Some("先写下文字，或选择一个文件")
        } else if self.send_mode == SendMode::Password {
            if self.send_password.is_empty() {
                Some("设置一个解密口令")
            } else if self.send_confirm.is_empty() {
                Some("请再次输入口令")
            } else if self.send_password != self.send_confirm {
                Some("两次口令不一致，请检查")
            } else {
                None
            }
        } else if self
            .get_active_recipient_key()
            .is_none_or(|key| validate_public_key(&key).is_err())
        {
            Some("请选择联系人，或粘贴有效的公开身份代码")
        } else {
            None
        }
    }


    fn get_active_recipient_key(&self) -> Option<String> {
        if let Some(idx) = self.selected_contact_idx {
            self.contacts.get(idx).map(|c| c.public_key.clone())
        } else {
            let key = self.recipient_custom_key.trim();
            if key.is_empty() {
                None
            } else {
                Some(key.to_string())
            }
        }
    }

    fn do_send(&mut self) {
        if let Some(hint) = self.send_blocker() {
            self.notify(hint, ToastKind::Warning);
            return;
        }
        let (bytes, meta, summary_kind) = match &self.send_file {
            Some(file) => (
                file.bytes.clone(),
                Some(FileMeta {
                    name: file.name.clone(),
                    extension: file.extension.clone(),
                    content_type: "application/octet-stream".into(),
                }),
                format!(
                    "文件: {}.{} ({})",
                    file.name,
                    file.extension,
                    format_file_size(file.size)
                ),
            ),
            None => {
                let text_bytes = self.send_text.as_bytes().to_vec();
                let count = self.send_text.chars().count();
                (text_bytes, None, format!("文字 ({count}字)"))
            }
        };

        if bytes.is_empty() {
            self.notify("内容不能为空", ToastKind::Warning);
            return;
        }

        let envelope = Envelope {
            kind: if meta.is_some() {
                "file".into()
            } else {
                "text".into()
            },
            meta,
            data: base64::engine::general_purpose::STANDARD.encode(&bytes),
        };

        let packet = match serde_json::to_vec(&envelope) {
            Ok(p) => p,
            Err(e) => {
                self.notify(format!("打包失败：{e}"), ToastKind::Error);
                return;
            }
        };

        let result = match self.send_mode {
            SendMode::Password => {
                if self.send_password.is_empty() || self.send_password != self.send_confirm {
                    self.notify("请填写相同的一致口令", ToastKind::Warning);
                    return;
                }
                encrypt_payload(&self.send_password, &packet)
            }
            SendMode::Contact => {
                let Some(pub_key) = self.get_active_recipient_key() else {
                    self.notify("请选择联系人或粘贴有效公钥", ToastKind::Warning);
                    return;
                };
                encrypt_for(&pub_key, &packet)
            }
        };

        match result {
            Ok(cipher) => {
                self.generated_cipher = cipher;
                let algo_name = match self.send_mode {
                    SendMode::Password => "使用口令接收",
                    SendMode::Contact => "联系人专属接收",
                };
                self.generated_summary = Some(format!("{summary_kind} · {algo_name}"));
                self.toast = None;
            }
            Err(e) => {
                self.notify(format!("加密失败：{e}"), ToastKind::Error);
            }
        }
    }

    fn do_receive(&mut self) {
        self.decrypted = None;
        let trimmed = self.receive_input.trim();
        if trimmed.is_empty() {
            self.notify("请先粘贴或打开密信", ToastKind::Warning);
            return;
        }

        let plain_bytes = if trimmed.starts_with(IDENTITY_PREFIX) {
            decrypt_with(&self.identity.0, trimmed)
        } else {
            decrypt_payload(&self.receive_password, trimmed)
        };

        match plain_bytes {
            Ok(bytes) => {
                // Check if it's Envelope json
                match serde_json::from_slice::<Envelope>(&bytes) {
                    Ok(packet) => {
                        match base64::engine::general_purpose::STANDARD.decode(&packet.data) {
                            Ok(data) => {
                                if packet.kind == "file" || packet.kind == "text-file" {
                                    let meta = packet.meta.unwrap_or(FileMeta {
                                        name: "解密文件".into(),
                                        extension: "bin".into(),
                                        content_type: "application/octet-stream".into(),
                                    });
                                    self.decrypted = Some(DecryptedContent::File {
                                        meta,
                                        bytes: data,
                                        saved_path: None,
                                    });
                                    self.notify("文件解密成功！请点击保存。", ToastKind::Success);
                                } else {
                                    let text = String::from_utf8_lossy(&data).into_owned();
                                    self.decrypted = Some(DecryptedContent::Text(text));
                                    self.notify("密信解密成功！", ToastKind::Success);
                                }
                            }
                            Err(_) => {
                                self.notify("密信数据包已损坏", ToastKind::Error);
                            }
                        }
                    }
                    Err(_) => {
                        // Old-version raw plaintext fallback
                        let text = String::from_utf8_lossy(&bytes).into_owned();
                        self.decrypted = Some(DecryptedContent::Text(text));
                        self.notify("已解密旧版文字密信", ToastKind::Success);
                    }
                }
            }
            Err(e) => {
                self.notify(format!("解密失败：{e}"), ToastKind::Error);
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Enforce theme
        if ctx.style().visuals.dark_mode != self.dark_mode {
            theme::apply_theme(ctx, self.dark_mode);
        }

        // Drag-and-drop handler
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped_files {
            if let Some(path) = file.path {
                if path.extension().is_some_and(|ext| ext == "mixin") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        self.tab = Tab::Receive;
                        self.receive_input = content.trim().to_string();
                        self.decrypted = None;
                        self.notify("已载入拖入的密信文件", ToastKind::Success);
                    }
                } else {
                    self.tab = Tab::Send;
                    self.load_file_for_send(path);
                }
            }
        }

        self.render_ui(ctx);

        // Repaint if toast is alive
        if self.toast.is_some() {
            ctx.request_repaint();
        }
    }
}

fn identity_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("mixin").join("identity.pem"))
}

fn theme_pref_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("mixin").join("theme.txt"))
}

fn load_theme_preference() -> bool {
    theme_pref_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .map(|s| s.trim() != "light")
        .unwrap_or(false)
}

fn save_theme_preference(dark_mode: bool) {
    if let Some(p) = theme_pref_path() {
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(p, if dark_mode { "dark" } else { "light" });
    }
}

fn load_identity() -> Option<(String, String)> {
    let private = fs::read_to_string(identity_path()?).ok()?;
    Some((private.clone(), public_from_private(&private).ok()?))
}

fn save_identity(private: &str) {
    if let Some(path) = identity_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if !path.exists() {
            let _ = fs::write(&path, private);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
            }
        }
    }
}

/// Persist an identity the user explicitly chose to replace (import or reset).
fn replace_identity(private: &str) -> Result<(), String> {
    let path = identity_path().ok_or("找不到应用数据目录")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, private).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([780.0, 540.0])
            .with_min_inner_size([720.0, 520.0])
            .with_title("密信 · 安全传递"),
        ..Default::default()
    };
    let initial_dark = load_theme_preference();
    eframe::run_native(
        "密信",
        options,
        Box::new(move |cc| {
            theme::configure_fonts(&cc.egui_ctx);
            theme::apply_theme(&cc.egui_ctx, initial_dark);
            Ok(Box::new(App::new()))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_roundtrip() {
        let (private, public) = generate_identity().unwrap();
        let plain = "中文文件测试".as_bytes();
        let token = encrypt_for(&public, plain).unwrap();
        assert_eq!(decrypt_with(&private, &token).unwrap(), plain);
    }

    #[test]
    fn envelope_text_roundtrip() {
        let text = "秘密会议时间：今晚八点。";
        let envelope = Envelope {
            kind: "text".into(),
            meta: None,
            data: base64::engine::general_purpose::STANDARD.encode(text.as_bytes()),
        };
        let packet = serde_json::to_vec(&envelope).unwrap();
        let cipher = encrypt_payload("my_secret_key", &packet).unwrap();

        let decrypted_bytes = decrypt_payload("my_secret_key", &cipher).unwrap();
        let dec_envelope: Envelope = serde_json::from_slice(&decrypted_bytes).unwrap();
        assert_eq!(dec_envelope.kind, "text");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(dec_envelope.data)
            .unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), text);
    }

    #[test]
    fn full_e2e_contact_flow() {
        // Alice generates identity
        let (alice_priv, _alice_pub) = generate_identity().unwrap();
        // Bob generates identity
        let (bob_priv, bob_pub) = generate_identity().unwrap();

        // Alice adds Bob to contacts
        let mut alice_contacts = Vec::new();
        let bob_contact = add_contact(&mut alice_contacts, "Bob", &bob_pub).unwrap();
        assert_eq!(bob_contact.name, "Bob");
        assert_eq!(bob_contact.fingerprint, compute_fingerprint(&bob_pub));

        // Alice encrypts a confidential message for Bob
        let secret_msg = "高度机密：明天上午9点准时签约。";
        let envelope = Envelope {
            kind: "text".into(),
            meta: None,
            data: base64::engine::general_purpose::STANDARD.encode(secret_msg.as_bytes()),
        };
        let packet = serde_json::to_vec(&envelope).unwrap();
        let cipher = encrypt_for(&bob_contact.public_key, &packet).unwrap();
        assert!(cipher.starts_with(IDENTITY_PREFIX));

        // Bob receives and decrypts with his private key
        let dec_bytes = decrypt_with(&bob_priv, &cipher).unwrap();
        let dec_env: Envelope = serde_json::from_slice(&dec_bytes).unwrap();
        let plain = base64::engine::general_purpose::STANDARD
            .decode(dec_env.data)
            .unwrap();
        assert_eq!(String::from_utf8(plain).unwrap(), secret_msg);

        // Alice attempts to decrypt with her own private key -> must fail!
        assert!(decrypt_with(&alice_priv, &cipher).is_err());
    }

    #[test]
    fn full_e2e_password_file_flow() {
        let file_data = b"ZIP mock payload 1234567890abcdef\x00\xFF";
        let meta = FileMeta {
            name: "archive".into(),
            extension: "zip".into(),
            content_type: "application/zip".into(),
        };
        let envelope = Envelope {
            kind: "file".into(),
            meta: Some(meta),
            data: base64::engine::general_purpose::STANDARD.encode(file_data),
        };
        let packet = serde_json::to_vec(&envelope).unwrap();
        let password = generate_memorable_password();

        // Encrypt with password
        let cipher = encrypt_payload(&password, &packet).unwrap();
        assert!(cipher.starts_with(PREFIX));

        // Decrypt with correct password
        let dec_bytes = decrypt_payload(&password, &cipher).unwrap();
        let dec_env: Envelope = serde_json::from_slice(&dec_bytes).unwrap();
        assert_eq!(dec_env.kind, "file");
        let meta = dec_env.meta.unwrap();
        assert_eq!(meta.name, "archive");
        assert_eq!(meta.extension, "zip");
        let restored_bytes = base64::engine::general_purpose::STANDARD
            .decode(dec_env.data)
            .unwrap();
        assert_eq!(&restored_bytes, file_data);

        // Decrypt with wrong password must fail
        assert!(decrypt_payload("wrong_password", &cipher).is_err());
    }
}

#[cfg(test)]
mod ui_tests;
