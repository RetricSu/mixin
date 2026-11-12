//! 火漆 (Waxseal) 桌面版：面向普通人的极简点对点安全传递工具
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod contacts;
mod crypto;
mod theme;
mod ui;

use base64::Engine as _;
use contacts::{add_contact, load_contacts, remove_contact, Contact};
use crypto::{
    compute_fingerprint, decrypt_payload, decrypt_with_sender, encrypt_for, encrypt_payload,
    format_file_size, format_fingerprint_spaced, generate_identity, generate_memorable_password,
    public_from_private, validate_public_key, Envelope, FileMeta, IDENTITY_PREFIX, PREFIX,
};
#[cfg(test)]
use crypto::decrypt_with;
use eframe::egui;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use theme::ThemeColors;

#[derive(PartialEq, Clone, Copy, Debug)]
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

#[derive(Clone, Debug)]
struct LoadedCipherFile {
    #[allow(dead_code)]
    path: Option<PathBuf>,
    name: String,
    size: usize,
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
    generated_show_password: bool,

    // --- 接收状态 ---
    receive_file: Option<LoadedCipherFile>,
    receive_input: String,
    receive_password: String,
    receive_show_password: bool,
    decrypted: Option<DecryptedContent>,
    verified_sender: Option<String>,
    clipboard_cipher: Option<String>,
    last_clipboard_check: Instant,
    clipboard_ignored: Option<String>,
    focus_receive_password: bool,

    // --- 隐私、窗口与快捷交互状态 ---
    window_focused: bool,
    privacy_blur_enabled: bool,
    manual_mask_hidden: bool,
    password_error_msg: Option<String>,
    password_error_at: Option<Instant>,
    show_qr_dialog: bool,

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
    editing_contact_idx: Option<usize>,
    editing_contact_name: String,
    confirm_overwrite_password: bool,
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
            generated_show_password: false,

            receive_file: None,
            receive_input: String::new(),
            receive_password: String::new(),
            receive_show_password: false,
            decrypted: None,
            verified_sender: None,
            clipboard_cipher: None,
            last_clipboard_check: Instant::now(),
            clipboard_ignored: None,
            focus_receive_password: false,

            window_focused: true,
            privacy_blur_enabled: true,
            manual_mask_hidden: false,
            password_error_msg: None,
            password_error_at: None,
            show_qr_dialog: false,

            identity,
            my_fingerprint,
            show_regenerate_confirm: false,
            contacts,
            new_contact_name: String::new(),
            new_contact_key: String::new(),
            show_add_contact_dialog: false,
            focus_new_contact: false,
            confirm_delete_idx: None,
            editing_contact_idx: None,
            editing_contact_name: String::new(),
            confirm_overwrite_password: false,
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

    fn load_cipher_file_for_receive(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(content) => {
                let trimmed = content.trim().to_string();
                if trimmed.is_empty() {
                    self.notify("密信文件内容为空", ToastKind::Warning);
                    return;
                }
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let size = match fs::metadata(&path) {
                    Ok(m) => m.len() as usize,
                    Err(_) => content.len(),
                };
                self.receive_file = Some(LoadedCipherFile {
                    name,
                    size,
                    path: Some(path),
                });
                self.receive_input = trimmed.clone();
                self.decrypted = None;
                self.receive_password.clear();
                if trimmed.starts_with(PREFIX) {
                    self.focus_receive_password = true;
                }
                self.tab = Tab::Receive;
                self.notify("已载入密信文件", ToastKind::Success);
            }
            Err(e) => {
                self.notify(format!("无法读取密信文件：{e}"), ToastKind::Error);
            }
        }
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
        let (bytes, meta, summary_kind) = match self.payload_type {
            PayloadType::File => match &self.send_file {
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
                    self.notify("请选择要加密的文件", ToastKind::Warning);
                    return;
                }
            },
            PayloadType::Text => {
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
                encrypt_for(&self.identity.0, &pub_key, &packet)
            }
        };

        match result {
            Ok(cipher) => {
                self.generated_cipher = cipher;
                self.generated_show_password = false;
                let algo_name = match self.send_mode {
                    SendMode::Password => "使用口令接收",
                    SendMode::Contact => "联系人专属接收 · Ed25519 已签名",
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
            match decrypt_with_sender(&self.identity.0, trimmed) {
                Ok((bytes, sender_fp)) => {
                    let sender_label = self
                        .contacts
                        .iter()
                        .find(|c| c.fingerprint == sender_fp)
                        .map(|c| format!("{} · 已验证签名", c.name))
                        .unwrap_or_else(|| {
                            if sender_fp == self.my_fingerprint {
                                "本机身份 · 已验证签名".to_string()
                            } else {
                                format!("指纹 {} · 已验证签名", sender_fp)
                            }
                        });
                    self.verified_sender = Some(sender_label);
                    Ok(bytes)
                }
                Err(e) => {
                    self.verified_sender = None;
                    Err(e)
                }
            }
        } else {
            self.verified_sender = None;
            decrypt_payload(&self.receive_password, trimmed)
        };

        match plain_bytes {
            Ok(bytes) => {
                self.password_error_msg = None;
                self.password_error_at = None;
                // Check if it's Envelope json
                match serde_json::from_slice::<Envelope>(&bytes) {
                    Ok(packet) => {
                        match base64::engine::general_purpose::STANDARD.decode(&packet.data) {
                            Ok(data) => {
                                if packet.kind == "file" || packet.kind == "text-file" {
                                    let meta = packet.meta.unwrap_or_else(|| {
                                        let (name, ext) = if let Some(rf) = &self.receive_file {
                                            let p = Path::new(&rf.name);
                                            let stem = p
                                                .file_stem()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .to_string();
                                            let ext = Path::new(&stem)
                                                .extension()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .to_string();
                                            (stem, ext)
                                        } else {
                                            ("解密文件".into(), "bin".into())
                                        };
                                        FileMeta {
                                            name,
                                            extension: ext,
                                            content_type: "application/octet-stream".into(),
                                        }
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
                        let is_binary = bytes.contains(&0);
                        if is_binary || self.receive_file.is_some() {
                            let (name, ext) = if let Some(rf) = &self.receive_file {
                                let p = Path::new(&rf.name);
                                let stem = p
                                    .file_stem()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                let ext = Path::new(&stem)
                                    .extension()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                (stem, ext)
                            } else {
                                ("解密文件".into(), "bin".into())
                            };
                            self.decrypted = Some(DecryptedContent::File {
                                meta: FileMeta {
                                    name,
                                    extension: ext,
                                    content_type: "application/octet-stream".into(),
                                },
                                bytes,
                                saved_path: None,
                            });
                            self.notify("文件解密成功！请点击保存。", ToastKind::Success);
                        } else {
                            // Old-version raw plaintext fallback
                            let text = String::from_utf8_lossy(&bytes).into_owned();
                            self.decrypted = Some(DecryptedContent::Text(text));
                            self.notify("已解密旧版文字密信", ToastKind::Success);
                        }
                    }
                }
            }
            Err(e) => {
                self.focus_receive_password = true;
                if trimmed.starts_with(PREFIX) {
                    self.password_error_msg = Some("口令不匹配".into());
                    self.password_error_at = Some(Instant::now());
                }
                self.notify(format!("解密失败：{e}"), ToastKind::Error);
            }
        }
    }

    pub(crate) fn destroy_decrypted(&mut self) {
        if let Some(mut content) = self.decrypted.take() {
            match &mut content {
                DecryptedContent::Text(ref mut s) => {
                    unsafe {
                        for b in s.as_mut_vec().iter_mut() {
                            *b = 0;
                        }
                    }
                    s.clear();
                }
                DecryptedContent::File { ref mut bytes, .. } => {
                    for b in bytes.iter_mut() {
                        *b = 0;
                    }
                    bytes.clear();
                }
            }
        }
        self.decrypted = None;
        self.verified_sender = None;
        self.receive_file = None;
        self.receive_input.clear();
        self.receive_password.clear();
        self.focus_receive_password = false;
        self.clipboard_cipher = None;
        self.manual_mask_hidden = false;
        self.password_error_msg = None;
        self.password_error_at = None;
        self.notify("解密内容已销毁，已恢复干净状态", ToastKind::Success);
    }

    pub(crate) fn delivery_guidance(&self, is_file: bool) -> String {
        match self.send_mode {
            SendMode::Password => {
                if is_file {
                    "我通过「火漆」向你发送了一个加密文件。\n\
                     【解密指引】\n\
                     1. 请使用「火漆」桌面端，将收到的 .seal 文件直接拖入软件或在接收页打开。\n\
                     2. 解密口令已通过单独渠道发送，输入口令即可解密并保存原文件。\n\
                     （安全提醒：密信与口令请分流发送，勿在同一会话中合并转发）"
                        .to_string()
                } else {
                    format!(
                        "我通过「火漆」向你发送了一条加密消息：\n\
                         ----------------------------------------\n\
                         {}\n\
                         ----------------------------------------\n\
                         【解密指引】\n\
                         1. 打开「火漆」客户端，进入「接收密信」页面（将自动识别剪贴板或手动粘贴）。\n\
                         2. 解密口令已通过单独安全渠道发送，输入口令即可解密。\n\
                         （安全提醒：密信与口令请分流发送，勿在同一会话中合并转发）",
                        self.generated_cipher
                    )
                }
            }
            SendMode::Contact => {
                if is_file {
                    "我通过「火漆」向你发送了一个专属加密文件。\n\
                     【解密指引】\n\
                     请使用「火漆」客户端打开该 .seal 文件，只有你的本机身份能够解开，无需口令。"
                        .to_string()
                } else {
                    format!(
                        "我通过「火漆」向你发送了一条专属加密消息：\n\
                         ----------------------------------------\n\
                         {}\n\
                         ----------------------------------------\n\
                         【解密指引】\n\
                         请在你的电脑上打开「火漆」客户端进入接收页，只有你的本机身份能够解开，无需口令。",
                        self.generated_cipher
                    )
                }
            }
        }
    }

    pub(crate) fn reveal_saved_file(&mut self) -> Result<(), String> {
        let path = match &self.decrypted {
            Some(DecryptedContent::File {
                saved_path: Some(path),
                ..
            }) => path.clone(),
            _ => {
                let err = "文件尚未保存到本地".to_string();
                self.notify(&err, ToastKind::Warning);
                return Err(err);
            }
        };
        match ui::reveal_in_file_manager(&path) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.notify(&e, ToastKind::Error);
                Err(e)
            }
        }
    }

    pub(crate) fn open_saved_file(&mut self) -> Result<(), String> {
        let path = match &self.decrypted {
            Some(DecryptedContent::File {
                saved_path: Some(path),
                ..
            }) => path.clone(),
            _ => {
                let err = "文件尚未保存到本地".to_string();
                self.notify(&err, ToastKind::Warning);
                return Err(err);
            }
        };
        match ui::open_file_native(&path) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.notify(&e, ToastKind::Error);
                Err(e)
            }
        }
    }

    pub(crate) fn check_clipboard_for_cipher(&mut self, ctx: &egui::Context) {
        if self.tab != Tab::Receive {
            return;
        }
        if !self.receive_input.trim().is_empty()
            || self.receive_file.is_some()
            || self.decrypted.is_some()
        {
            self.clipboard_cipher = None;
            return;
        }

        let is_window_focused = ctx.input(|i| {
            i.raw
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::WindowFocused(true)))
        });
        let elapsed = self.last_clipboard_check.elapsed();
        if is_window_focused || elapsed >= std::time::Duration::from_millis(800) {
            self.last_clipboard_check = Instant::now();
            self.poll_clipboard();
        }
    }

    pub(crate) fn poll_clipboard(&mut self) {
        if !self.receive_input.trim().is_empty()
            || self.receive_file.is_some()
            || self.decrypted.is_some()
        {
            self.clipboard_cipher = None;
            return;
        }
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if let Ok(text) = cb.get_text() {
                let trimmed = text.trim();
                if (trimmed.starts_with(PREFIX) || trimmed.starts_with(IDENTITY_PREFIX))
                    && trimmed.len() <= 10_000_000
                {
                    if self.clipboard_ignored.as_deref() != Some(trimmed) {
                        self.clipboard_cipher = Some(trimmed.to_string());
                    }
                } else {
                    self.clipboard_cipher = None;
                    if self.clipboard_ignored.is_some() {
                        self.clipboard_ignored = None;
                    }
                }
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
                let is_cipher_ext = path.extension().is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("seal") || ext.eq_ignore_ascii_case("mixin")
                });
                if is_cipher_ext {
                    self.load_cipher_file_for_receive(path);
                } else if self.tab == Tab::Receive {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let trimmed = content.trim();
                        if trimmed.starts_with(PREFIX) || trimmed.starts_with(IDENTITY_PREFIX) {
                            self.load_cipher_file_for_receive(path);
                            continue;
                        }
                    }
                    self.tab = Tab::Send;
                    self.load_file_for_send(path);
                } else {
                    self.tab = Tab::Send;
                    self.load_file_for_send(path);
                }
            }
        }

        self.render_ui(ctx);

        if self.tab == Tab::Receive {
            self.check_clipboard_for_cipher(ctx);
        }

        // Repaint if toast is alive or password shake animation is active
        let is_shaking = self
            .password_error_at
            .is_some_and(|t| t.elapsed().as_secs_f32() < 0.4);
        if self.toast.is_some() || is_shaking {
            ctx.request_repaint();
        }
    }
}

fn identity_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("waxseal").join("identity.pem"))
}

fn legacy_identity_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("mixin").join("identity.pem"))
}

fn theme_pref_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("waxseal").join("theme.txt"))
}

fn legacy_theme_pref_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("mixin").join("theme.txt"))
}

fn load_theme_preference_from(primary: Option<&PathBuf>, legacy: Option<&PathBuf>) -> bool {
    if let Some(p) = primary.filter(|p| p.exists()) {
        return fs::read_to_string(p)
            .map(|s| s.trim() != "light")
            .unwrap_or(false);
    }
    if let Some(old_p) = legacy.filter(|p| p.exists()) {
        if let Ok(content) = fs::read_to_string(old_p) {
            if let Some(p) = primary {
                if let Some(parent) = p.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(p, &content);
            }
            return content.trim() != "light";
        }
    }
    false
}

fn load_theme_preference() -> bool {
    load_theme_preference_from(theme_pref_path().as_ref(), legacy_theme_pref_path().as_ref())
}

fn save_theme_preference(dark_mode: bool) {
    if let Some(p) = theme_pref_path() {
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(p, if dark_mode { "dark" } else { "light" });
    }
}

fn load_identity_from(
    primary: Option<&PathBuf>,
    legacy: Option<&PathBuf>,
) -> Option<(String, String)> {
    let path = primary?;
    if path.exists() {
        let private = fs::read_to_string(path).ok()?;
        return Some((private.clone(), public_from_private(&private).ok()?));
    }
    if let Some(old_path) = legacy.filter(|p| p.exists()) {
        let private = fs::read_to_string(old_path).ok()?;
        let pub_pem = public_from_private(&private).ok()?;
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::write(path, &private).is_ok() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
            }
        }
        return Some((private, pub_pem));
    }
    None
}

fn load_identity() -> Option<(String, String)> {
    load_identity_from(identity_path().as_ref(), legacy_identity_path().as_ref())
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
            .with_title("火漆 · 安全传递"),
        ..Default::default()
    };
    let initial_dark = load_theme_preference();
    eframe::run_native(
        "火漆",
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
        let token = encrypt_for(&private, &public, plain).unwrap();
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
        let (alice_priv, alice_pub) = generate_identity().unwrap();
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
        let cipher = encrypt_for(&alice_priv, &bob_contact.public_key, &packet).unwrap();
        assert!(cipher.starts_with(IDENTITY_PREFIX));

        // Bob receives and decrypts with his private key, verifying sender
        let (dec_bytes, sender_fp) = decrypt_with_sender(&bob_priv, &cipher).unwrap();
        assert_eq!(sender_fp, compute_fingerprint(&alice_pub));
        let dec_env: Envelope = serde_json::from_slice(&dec_bytes).unwrap();
        let plain = base64::engine::general_purpose::STANDARD
            .decode(dec_env.data)
            .unwrap();
        assert_eq!(String::from_utf8(plain).unwrap(), secret_msg);

        // Alice attempts to decrypt with her own private key -> must fail!
        assert!(decrypt_with(&alice_priv, &cipher).is_err());

        // Test App level verified_sender display logic
        let mut bob_app = App::with_identity(
            (bob_priv.clone(), bob_pub.clone()),
            vec![Contact {
                name: "Alice".into(),
                public_key: alice_pub.clone(),
                fingerprint: compute_fingerprint(&alice_pub),
                created_at: "2025-01-01".into(),
            }],
            false,
        );
        bob_app.receive_input = cipher.clone();
        bob_app.do_receive();
        assert_eq!(
            bob_app.verified_sender,
            Some("Alice · 已验证签名".to_string()),
            "Known contact must be resolved by name and verified"
        );

        // Unknown sender (Eve)
        let (eve_priv, eve_pub) = generate_identity().unwrap();
        let eve_cipher = encrypt_for(&eve_priv, &bob_pub, &packet).unwrap();
        bob_app.receive_input = eve_cipher;
        bob_app.do_receive();
        assert_eq!(
            bob_app.verified_sender,
            Some(format!("指纹 {} · 已验证签名", compute_fingerprint(&eve_pub))),
            "Unknown contact must display verified fingerprint"
        );

        // Destroy resets verified_sender
        bob_app.destroy_decrypted();
        assert!(bob_app.verified_sender.is_none());
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

    #[test]
    fn legacy_identity_and_theme_migration() {
        let base = std::env::temp_dir().join(format!("waxseal_app_mig_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let legacy_id = base.join("mixin").join("identity.pem");
        let new_id = base.join("waxseal").join("identity.pem");
        let legacy_theme = base.join("mixin").join("theme.txt");
        let new_theme = base.join("waxseal").join("theme.txt");
        fs::create_dir_all(legacy_id.parent().unwrap()).unwrap();

        let (priv_pem, pub_pem) = generate_identity().unwrap();
        let legacy_priv = priv_pem
            .replace(
                crypto::PEM_PRIVATE_HEADER,
                "-----BEGIN MIXIN PRIVATE IDENTITY-----",
            )
            .replace(
                crypto::PEM_PRIVATE_FOOTER,
                "-----END MIXIN PRIVATE IDENTITY-----",
            );
        fs::write(&legacy_id, &legacy_priv).unwrap();
        fs::write(&legacy_theme, "dark").unwrap();

        let migrated = load_identity_from(Some(&new_id), Some(&legacy_id)).unwrap();
        assert_eq!(migrated.0, legacy_priv);
        assert_eq!(migrated.1, pub_pem);
        assert!(new_id.exists());

        assert!(load_theme_preference_from(Some(&new_theme), Some(&legacy_theme)));
        assert!(new_theme.exists());

        let _ = fs::remove_dir_all(&base);
    }
}

#[cfg(test)]
mod ui_tests;
