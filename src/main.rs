//! 密信桌面版：面向普通人的极简点对点安全传递工具
mod contacts;
mod crypto;
mod theme;

use base64::Engine as _;
use contacts::{add_contact, load_contacts, remove_contact, Contact};
use crypto::{
    compute_fingerprint, decrypt_payload, decrypt_with, encrypt_for, encrypt_payload,
    format_file_size, generate_identity, generate_memorable_password, public_from_private,
    validate_public_key, Envelope, FileMeta, IDENTITY_PREFIX, PREFIX,
};
use eframe::egui::{self, Color32, Margin, Rounding, Stroke};
use std::{
    fs,
    path::PathBuf,
    time::Instant,
};
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
}

impl App {
    fn new() -> Self {
        let identity =
            load_identity().unwrap_or_else(|| generate_identity().expect("无法生成本机身份"));
        save_identity(&identity.0);
        let my_fingerprint = compute_fingerprint(&identity.1);
        let contacts = load_contacts();

        let dark_mode = load_theme_preference();

        Self {
            tab: Tab::Send,
            dark_mode,
            toast: Some(Toast {
                message: "本机端对端安全环境已就绪".to_string(),
                kind: ToastKind::Success,
                created_at: Instant::now(),
                duration_secs: 2.5,
            }),

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
            kind,
            created_at: Instant::now(),
            duration_secs: 3.5,
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
                self.notify("文件载入成功", ToastKind::Success);
            }
            Err(e) => {
                self.notify(format!("读取文件失败：{e}"), ToastKind::Error);
            }
        }
    }

    // --- UI 渲染主入口 ---
    fn render_header(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        egui::Frame::none()
            .fill(t.bg_header)
            .inner_margin(Margin::symmetric(24.0, 14.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Logo & App Name
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, Rounding::same(6.0), t.accent_emerald);
                        painter.text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "密",
                            egui::FontId::new(16.0, egui::FontFamily::Proportional),
                            Color32::WHITE,
                        );

                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("密信")
                                .size(18.0)
                                .color(t.text_primary)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("端对端安全传递")
                                .size(11.5)
                                .color(t.text_muted),
                        );
                    });

                    ui.add_space(24.0);

                    // Center Tabs: strictly left-to-right
                    self.render_tabs(ui);

                    // Right Status & Theme toggle
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let theme_text = if self.dark_mode { "☀ 浅色" } else { "🌙 深色" };
                        let theme_tip = if self.dark_mode { "切换为明亮浅色主题" } else { "切换为极简深色主题" };
                        if ui
                            .button(egui::RichText::new(theme_text).size(12.0))
                            .on_hover_text(theme_tip)
                            .clicked()
                        {
                            self.dark_mode = !self.dark_mode;
                            save_theme_preference(self.dark_mode);
                            theme::apply_theme(ui.ctx(), self.dark_mode);
                        }

                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot_rect.center(), 4.0, t.accent_emerald);
                            ui.label(
                                egui::RichText::new("安全环境正常")
                                    .size(11.5)
                                    .color(t.text_muted),
                            );
                        });
                    });
                });
            });
    }

    fn render_tabs(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        let tabs = [
            (Tab::Send, "📤 发送密信"),
            (Tab::Receive, "📥 接收解密"),
            (Tab::Identity, "👤 身份与联系人"),
        ];

        egui::Frame::none()
            .fill(t.bg_well)
            .stroke(Stroke::new(1.0, t.border_card))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(4.0, 3.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (tab_val, label) in tabs {
                        let is_selected = self.tab == tab_val;
                        let text = egui::RichText::new(label)
                            .size(13.0)
                            .color(if is_selected {
                                if self.dark_mode { Color32::WHITE } else { t.accent_emerald }
                            } else {
                                t.text_secondary
                            })
                            .strong();

                        let fill = if is_selected {
                            t.bg_card_hover
                        } else {
                            Color32::TRANSPARENT
                        };

                        let stroke = if is_selected {
                            Stroke::new(1.0, t.border_card_light)
                        } else {
                            Stroke::NONE
                        };

                        let btn = egui::Button::new(text)
                            .fill(fill)
                            .stroke(stroke)
                            .rounding(Rounding::same(6.0))
                            .min_size(egui::vec2(96.0, 26.0));

                        if ui.add(btn).clicked() {
                            self.tab = tab_val;
                        }
                    }
                });
            });
    }

    fn render_toast(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        let Some(toast) = self.toast.clone() else { return };
        let elapsed = toast.created_at.elapsed().as_secs_f32();
        if elapsed > toast.duration_secs {
            self.toast = None;
            return;
        }

        let (border_color, tag) = match toast.kind {
            ToastKind::Success => (t.accent_emerald, "成功"),
            ToastKind::Info => (t.accent_sky, "提示"),
            ToastKind::Warning => (t.accent_amber, "注意"),
            ToastKind::Error => (t.accent_rose, "错误"),
        };

        let mut dismiss = false;
        egui::Frame::none()
            .fill(t.bg_card_elevated)
            .stroke(Stroke::new(1.5, border_color))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::symmetric(16.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::pill_badge(ui, tag, border_color);
                    ui.label(
                        egui::RichText::new(&toast.message)
                            .color(t.text_primary)
                            .size(13.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(egui::RichText::new("✕").size(11.0).color(t.text_muted))
                            .clicked()
                        {
                            dismiss = true;
                        }
                    });
                });
            });
        if dismiss {
            self.toast = None;
        }
        ui.add_space(8.0);
    }

    // ========================================================
    // TAB 1: 发送密信
    // ========================================================
    fn render_send_tab(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Mode selection card
            self.render_send_mode_selector(ui);
            ui.add_space(12.0);

            // Recipient selection (if Contact mode)
            if self.send_mode == SendMode::Contact {
                self.render_recipient_selector(ui);
                ui.add_space(12.0);
            }

            // Payload input (Text vs File)
            self.render_payload_input(ui);
            ui.add_space(12.0);

            // Password input (if Password mode)
            if self.send_mode == SendMode::Password {
                self.render_password_input(ui);
                ui.add_space(12.0);
            }

            // Encrypt action button
            let can_encrypt = match self.send_mode {
                SendMode::Password => {
                    !self.send_password.is_empty()
                        && self.send_password == self.send_confirm
                        && (self.send_file.is_some() || !self.send_text.trim().is_empty())
                }
                SendMode::Contact => {
                    let has_key = self.get_active_recipient_key().is_some();
                    has_key && (self.send_file.is_some() || !self.send_text.trim().is_empty())
                }
            };

            ui.horizontal(|ui| {
                let action_btn = egui::Button::new(
                    egui::RichText::new("🔒 生成加密密信")
                        .size(15.0)
                        .color(if can_encrypt {
                            Color32::WHITE
                        } else {
                            t.text_muted
                        })
                        .strong(),
                )
                .fill(if can_encrypt {
                    t.accent_emerald
                } else {
                    t.bg_card
                })
                .min_size(egui::vec2(160.0, 38.0))
                .rounding(Rounding::same(8.0));

                if ui.add_enabled(can_encrypt, action_btn).clicked() {
                    self.do_send();
                }

                if !can_encrypt {
                    let hint = match self.send_mode {
                        SendMode::Password => {
                            if self.send_file.is_none() && self.send_text.trim().is_empty() {
                                "请填写要传递的文字或选择文件"
                            } else if self.send_password.is_empty() {
                                "请设置口令"
                            } else if self.send_password != self.send_confirm {
                                "两次输入的口令不一致"
                            } else {
                                ""
                            }
                        }
                        SendMode::Contact => {
                            if self.send_file.is_none() && self.send_text.trim().is_empty() {
                                "请填写要传递的文字或选择文件"
                            } else {
                                "请选择联系人或粘贴有效公钥"
                            }
                        }
                    };
                    ui.label(egui::RichText::new(hint).size(12.0).color(t.accent_amber));
                }
            });

            // Result preview section
            if !self.generated_cipher.is_empty() {
                ui.add_space(14.0);
                self.render_generated_output(ui);
            }
        });
    }

    fn render_send_mode_selector(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card_frame(&t).show(ui, |ui| {
            ui.label(
                egui::RichText::new("第一步：选择加密方式")
                    .size(14.0)
                    .color(t.text_primary)
                    .strong(),
            );
            ui.add_space(8.0);

            ui.columns(2, |cols| {
                // Option A: Password
                let is_pwd = self.send_mode == SendMode::Password;
                let frame_a = if is_pwd {
                    theme::highlighted_card_frame(&t, t.accent_emerald)
                } else {
                    theme::card_frame(&t)
                };

                let res_a = frame_a.show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🔑 约定一个口令")
                                .size(14.5)
                                .color(if is_pwd { t.accent_emerald } else { t.text_primary })
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if is_pwd {
                                theme::pill_badge(ui, "已选择", t.accent_emerald);
                            } else {
                                theme::pill_badge(ui, "对称加密", t.text_muted);
                            }
                        });
                    });
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("使用双方约定的密码进行 AES-256 加密。适合临时沟通，通过电话或微信告知口令。")
                            .size(11.5)
                            .color(t.text_secondary),
                    );
                });

                if cols[0].interact(res_a.response.rect, res_a.response.id, egui::Sense::click()).clicked() {
                    self.send_mode = SendMode::Password;
                }

                // Option B: Contact
                let is_contact = self.send_mode == SendMode::Contact;
                let frame_b = if is_contact {
                    theme::highlighted_card_frame(&t, t.accent_emerald)
                } else {
                    theme::card_frame(&t)
                };

                let res_b = frame_b.show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("👤 联系人专属加密")
                                .size(14.5)
                                .color(if is_contact { t.accent_emerald } else { t.text_primary })
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if is_contact {
                                theme::pill_badge(ui, "已选择", t.accent_emerald);
                            } else {
                                theme::pill_badge(ui, "非对称加密", t.text_muted);
                            }
                        });
                    });
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("使用对方的公开身份加密，仅对方本机能解开。适合长期联系人，无需每次约定密码。")
                            .size(11.5)
                            .color(t.text_secondary),
                    );
                });

                if cols[1].interact(res_b.response.rect, res_b.response.id, egui::Sense::click()).clicked() {
                    self.send_mode = SendMode::Contact;
                }
            });
        });
    }

    fn render_recipient_selector(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card_frame(&t).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("指定接收方：")
                        .size(14.0)
                        .color(t.text_primary)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("＋ 新增常用联系人").size(12.0)).clicked() {
                        self.tab = Tab::Identity;
                        self.show_add_contact_dialog = true;
                    }
                });
            });
            ui.add_space(8.0);

            // Contact chips
            if !self.contacts.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for (i, contact) in self.contacts.iter().enumerate() {
                        let is_selected = self.selected_contact_idx == Some(i);
                        let chip_text = format!("👤 {}", contact.name);
                        let chip = egui::Button::new(
                            egui::RichText::new(chip_text)
                                .size(12.5)
                                .color(if is_selected {
                                    Color32::WHITE
                                } else {
                                    t.text_primary
                                }),
                        )
                        .fill(if is_selected {
                            t.accent_emerald
                        } else {
                            t.bg_card_elevated
                        })
                        .stroke(if is_selected {
                            Stroke::new(1.0, t.accent_emerald)
                        } else {
                            Stroke::new(1.0, t.border_card_light)
                        })
                        .rounding(Rounding::same(14.0));

                        if ui.add(chip).clicked() {
                            self.selected_contact_idx = if is_selected { None } else { Some(i) };
                        }
                    }

                    let is_custom = self.selected_contact_idx.is_none();
                    let custom_chip = egui::Button::new(
                        egui::RichText::new("📋 临时粘贴公钥")
                            .size(12.5)
                            .color(if is_custom {
                                if self.dark_mode { Color32::WHITE } else { t.accent_emerald }
                            } else {
                                t.text_muted
                            }),
                    )
                    .fill(if is_custom {
                        t.bg_card_hover
                    } else {
                        t.bg_well
                    })
                    .stroke(Stroke::new(1.0, t.border_card))
                    .rounding(Rounding::same(14.0));

                    if ui.add(custom_chip).clicked() {
                        self.selected_contact_idx = None;
                    }
                });
                ui.add_space(8.0);
            }

            if let Some(idx) = self.selected_contact_idx {
                if let Some(c) = self.contacts.get(idx) {
                    ui.horizontal(|ui| {
                        theme::pill_badge(ui, &format!("已选：{}", c.name), t.accent_emerald);
                        ui.label(
                            egui::RichText::new(format!("指纹: {}", c.fingerprint))
                                .size(12.0)
                                .color(t.text_muted),
                        );
                    });
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("粘贴对方的公开身份代码：")
                            .size(12.5)
                            .color(t.text_secondary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(egui::RichText::new("📋 从剪贴板粘贴").size(12.0)).clicked() {
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                if let Ok(text) = cb.get_text() {
                                    self.recipient_custom_key = text;
                                }
                            }
                        }
                    });
                });
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.recipient_custom_key)
                        .hint_text("-----BEGIN PUBLIC KEY-----\n...")
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY),
                );

                if !self.recipient_custom_key.trim().is_empty() {
                    match validate_public_key(&self.recipient_custom_key) {
                        Ok(fp) => {
                            ui.horizontal(|ui| {
                                theme::pill_badge(ui, "有效公钥", t.accent_emerald);
                                ui.label(
                                    egui::RichText::new(format!("公钥指纹: {fp}"))
                                        .size(12.0)
                                        .color(t.text_muted),
                                );
                            });
                        }
                        Err(e) => {
                            ui.label(egui::RichText::new(format!("⚠️ {e}")).size(12.0).color(t.accent_rose));
                        }
                    }
                }
            }
        });
    }

    fn render_payload_input(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card_frame(&t).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("第二步：准备传递内容")
                        .size(14.0)
                        .color(t.text_primary)
                        .strong(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.send_file.is_some() {
                        if ui
                            .button(egui::RichText::new("✖ 移除文件并换回文字").size(12.0).color(t.accent_rose))
                            .clicked()
                        {
                            self.send_file = None;
                            self.payload_type = PayloadType::Text;
                        }
                    } else if ui.button(egui::RichText::new("📂 选择文件...").size(12.0)).clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.load_file_for_send(path);
                        }
                    }
                });
            });
            ui.add_space(8.0);

            let mut reselect_path = None;
            if let Some(file) = self.send_file.clone() {
                // Show File Card
                theme::well_frame(&t).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::hover());
                        let painter = ui.painter_at(icon_rect);
                        painter.rect_filled(icon_rect, Rounding::same(6.0), t.bg_card_elevated);
                        painter.text(
                            icon_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "📄",
                            egui::FontId::new(20.0, egui::FontFamily::Proportional),
                            Color32::WHITE,
                        );

                        ui.vertical(|ui| {
                            let display_name = if file.extension.is_empty() {
                                file.name.clone()
                            } else {
                                format!("{}.{}", file.name, file.extension)
                            };
                            ui.label(
                                egui::RichText::new(display_name)
                                    .size(14.0)
                                    .color(t.text_primary)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "大小: {} · 文件已加载",
                                    format_file_size(file.size)
                                ))
                                .size(11.5)
                                .color(t.text_muted),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(egui::RichText::new("重新选择").size(12.0)).clicked() {
                                reselect_path = rfd::FileDialog::new().pick_file();
                            }
                        });
                    });
                });
                if let Some(path) = reselect_path {
                    self.load_file_for_send(path);
                }
            } else {
                // Show Text Area
                ui.add(
                    egui::TextEdit::multiline(&mut self.send_text)
                        .hint_text("在此写下或粘贴需要保密的内容（如密码、账号、通讯录、地址、私人备忘等）…\n也可点击右上角选择任意文件发送。")
                        .desired_rows(6)
                        .desired_width(f32::INFINITY),
                );

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("共 {} 字", self.send_text.chars().count()))
                            .size(11.5)
                            .color(t.text_muted),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.send_text.is_empty()
                            && ui.button(egui::RichText::new("清空").size(11.5)).clicked()
                        {
                            self.send_text.clear();
                        }
                        if ui.button(egui::RichText::new("📋 粘贴文字").size(11.5)).clicked() {
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                if let Ok(t_str) = cb.get_text() {
                                    self.send_text.push_str(&t_str);
                                }
                            }
                        }
                    });
                });
            }
        });
    }

    fn render_password_input(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card_frame(&t).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("第三步：设置解密口令")
                        .size(14.0)
                        .color(t.text_primary)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("🎲 随机生成安全口令").size(12.0)).clicked() {
                        let pwd = generate_memorable_password();
                        self.send_password = pwd.clone();
                        self.send_confirm = pwd.clone();
                        self.copy_text(ui.ctx(), pwd, "已生成并复制口令！对方解密时需要此口令。");
                    }
                });
            });
            ui.add_space(8.0);

            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    ui.label(egui::RichText::new("输入口令：").size(12.5).color(t.text_secondary));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.send_password)
                            .password(!self.send_show_password)
                            .desired_width(f32::INFINITY),
                    );
                });

                cols[1].vertical(|ui| {
                    ui.label(egui::RichText::new("再次确认口令：").size(12.5).color(t.text_secondary));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.send_confirm)
                            .password(!self.send_show_password)
                            .desired_width(f32::INFINITY),
                    );
                });
            });

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.send_show_password, egui::RichText::new("显示口令明文").size(12.0));

                if !self.send_password.is_empty() && !self.send_confirm.is_empty() {
                    if self.send_password == self.send_confirm {
                        ui.label(egui::RichText::new("✓ 两次口令一致").size(12.0).color(t.accent_emerald));
                    } else {
                        ui.label(egui::RichText::new("⚠ 口令不匹配").size(12.0).color(t.accent_rose));
                    }
                }
            });
        });
    }

    fn render_generated_output(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::highlighted_card_frame(&t, t.accent_emerald).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("🎉 密信生成成功！")
                        .size(15.0)
                        .color(t.accent_emerald)
                        .strong(),
                );
                if let Some(summary) = &self.generated_summary {
                    ui.label(egui::RichText::new(summary).size(12.0).color(t.text_muted));
                }
            });
            ui.add_space(6.0);

            // Preview well
            theme::well_frame(&t).show(ui, |ui| {
                let preview_len = self.generated_cipher.chars().count();
                let display_preview = if preview_len > 180 {
                    let first_part: String = self.generated_cipher.chars().take(160).collect();
                    format!("{first_part}...\n[密信全文共 {preview_len} 字符]")
                } else {
                    self.generated_cipher.clone()
                };

                ui.label(
                    egui::RichText::new(display_preview)
                        .monospace()
                        .size(11.5)
                        .color(t.text_secondary),
                );
            });
            ui.add_space(8.0);

            // Dual action buttons
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("📋 一键复制密信文本")
                                .size(13.5)
                                .color(Color32::WHITE)
                                .strong(),
                        )
                        .fill(t.accent_emerald)
                        .min_size(egui::vec2(150.0, 34.0))
                        .rounding(Rounding::same(8.0)),
                    )
                    .clicked()
                {
                    self.copy_text(ui.ctx(), self.generated_cipher.clone(), "密信文本已复制到剪贴板！可直接发送给对方。");
                }

                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("💾 另存为密信文件 (.mixin)")
                                .size(13.5)
                                .color(t.text_primary),
                        )
                        .fill(t.bg_card_elevated)
                        .stroke(Stroke::new(1.0, t.border_card_light))
                        .min_size(egui::vec2(180.0, 34.0))
                        .rounding(Rounding::same(8.0)),
                    )
                    .clicked()
                {
                    let default_name = if let Some(file) = &self.send_file {
                        format!("{}.mixin", file.name)
                    } else {
                        "密信.mixin".to_string()
                    };
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(default_name)
                        .add_filter("密信文件", &["mixin"])
                        .save_file()
                    {
                        match fs::write(&path, &self.generated_cipher) {
                            Ok(_) => {
                                self.notify(
                                    format!("已保存至：{}", path.file_name().unwrap_or_default().to_string_lossy()),
                                    ToastKind::Success,
                                );
                            }
                            Err(e) => {
                                self.notify(format!("保存文件失败：{e}"), ToastKind::Error);
                            }
                        }
                    }
                }
            });

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("💡 安全提醒：密信与口令请分不同渠道发送（如微信发密信文件，电话告知口令），切勿写在同一处。")
                    .size(11.5)
                    .color(t.text_muted),
            );
        });
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
        let (bytes, meta, summary_kind) = match &self.send_file {
            Some(file) => (
                file.bytes.clone(),
                Some(FileMeta {
                    name: file.name.clone(),
                    extension: file.extension.clone(),
                    content_type: "application/octet-stream".into(),
                }),
                format!("文件: {}.{} ({})", file.name, file.extension, format_file_size(file.size)),
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
                    SendMode::Password => "口令加密 (AES-256)",
                    SendMode::Contact => "联系人专属 (RSA-2048 + AES)",
                };
                self.generated_summary = Some(format!("{summary_kind} · {algo_name}"));
                self.notify("密信生成成功！", ToastKind::Success);
            }
            Err(e) => {
                self.notify(format!("加密失败：{e}"), ToastKind::Error);
            }
        }
    }

    // ========================================================
    // TAB 2: 接收解密
    // ========================================================
    fn render_receive_tab(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Input card
            theme::card_frame(&t).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("输入收到的密信")
                            .size(14.0)
                            .color(t.text_primary)
                            .strong(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.receive_input.is_empty()
                            && ui.button(egui::RichText::new("清空").size(12.0)).clicked()
                        {
                            self.receive_input.clear();
                            self.decrypted = None;
                        }

                        if ui.button(egui::RichText::new("📂 打开密信文件...").size(12.0)).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("密信文件", &["mixin", "txt"])
                                .pick_file()
                            {
                                if let Ok(content) = fs::read_to_string(&path) {
                                    self.receive_input = content.trim().to_string();
                                    self.notify("密信文件已载入", ToastKind::Success);
                                }
                            }
                        }

                        if ui.button(egui::RichText::new("📋 从剪贴板粘贴").size(12.0)).clicked() {
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                if let Ok(t_str) = cb.get_text() {
                                    self.receive_input = t_str.trim().to_string();
                                    self.notify("已从剪贴板粘贴", ToastKind::Success);
                                }
                            }
                        }
                    });
                });
                ui.add_space(8.0);

                ui.add(
                    egui::TextEdit::multiline(&mut self.receive_input)
                        .hint_text("在此粘贴收到的密信内容（以 MX1. 或 MXID1. 开头），或拖拽 .mixin 文件至窗口…")
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(5)
                        .desired_width(f32::INFINITY),
                );
            });
            ui.add_space(12.0);

            // Auto-detect mode & decryption trigger
            let trimmed = self.receive_input.trim();
            if trimmed.is_empty() {
                self.render_receive_help(ui);
            } else if trimmed.starts_with(IDENTITY_PREFIX) {
                self.render_identity_decrypt_card(ui);
            } else if trimmed.starts_with(PREFIX) {
                self.render_password_decrypt_card(ui);
            } else {
                theme::card_frame(&t).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("⚠️ 未识别的密信格式")
                                .size(13.5)
                                .color(t.accent_amber)
                                .strong(),
                        );
                    });
                    ui.label(
                        egui::RichText::new("有效的密信通常以 MX1. (口令加密) 或 MXID1. (专属加密) 开头。请检查是否复制完整。")
                            .size(12.0)
                            .color(t.text_muted),
                    );
                });
            }

            // Decrypted content presentation
            if self.decrypted.is_some() {
                ui.add_space(14.0);
                self.render_decrypted_result(ui);
            }
        });
    }

    fn render_receive_help(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card_frame(&t).show(ui, |ui| {
            ui.label(
                egui::RichText::new("💡 如何解密？")
                    .size(13.5)
                    .color(t.text_primary)
                    .strong(),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("1. 点击右上角“从剪贴板粘贴”，或直接拖入收到的 .mixin 文件。\n2. 系统将自动识别是“口令加密”还是“联系人专属”。\n3. 专属密信可一键使用本机身份解密；口令密信输入约定口令即可解开。")
                    .size(12.0)
                    .color(t.text_muted),
            );
        });
    }

    fn render_identity_decrypt_card(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::highlighted_card_frame(&t, t.accent_sky).show(ui, |ui| {
            ui.horizontal(|ui| {
                theme::pill_badge(ui, "联系人专属密信", t.accent_sky);
                ui.label(
                    egui::RichText::new("检测到该密信已绑定本机公开身份")
                        .size(13.0)
                        .color(t.text_primary),
                );
            });
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("此密信由对方使用你的公开公钥加密，无需口令，点击下方按钮将自动调用本机私钥解开。")
                    .size(12.0)
                    .color(t.text_muted),
            );
            ui.add_space(8.0);

            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("🔓 一键使用本机身份解密")
                            .size(14.0)
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(t.accent_sky)
                    .min_size(egui::vec2(160.0, 36.0))
                    .rounding(Rounding::same(8.0)),
                )
                .clicked()
            {
                self.do_receive();
            }
        });
    }

    fn render_password_decrypt_card(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::highlighted_card_frame(&t, t.accent_amber).show(ui, |ui| {
            ui.horizontal(|ui| {
                theme::pill_badge(ui, "口令保护密信", t.accent_amber);
                ui.label(
                    egui::RichText::new("此密信受口令保护，请输入双方约定的口令：")
                        .size(13.0)
                        .color(t.text_primary),
                );
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.receive_password)
                        .password(!self.receive_show_password)
                        .hint_text("输入解密口令…")
                        .desired_width(260.0),
                );

                ui.checkbox(&mut self.receive_show_password, egui::RichText::new("显示").size(12.0));

                let can_decrypt = !self.receive_password.is_empty();
                let btn = egui::Button::new(
                    egui::RichText::new("🔓 解密密信")
                        .size(13.5)
                        .color(if can_decrypt {
                            Color32::WHITE
                        } else {
                            t.text_muted
                        })
                        .strong(),
                )
                .fill(if can_decrypt {
                    t.accent_emerald
                } else {
                    t.bg_card
                })
                .min_size(egui::vec2(110.0, 32.0))
                .rounding(Rounding::same(7.0));

                if ui.add_enabled(can_decrypt, btn).clicked() {
                    self.do_receive();
                }
            });
        });
    }

    fn render_decrypted_result(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        let Some(content) = self.decrypted.clone() else { return };

        theme::highlighted_card_frame(&t, t.accent_emerald).show(ui, |ui| {
            match content {
                DecryptedContent::Text(plain_text) => {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("✅ 解密成功：文字内容")
                                .size(15.0)
                                .color(t.accent_emerald)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!("共 {} 字", plain_text.chars().count()))
                                .size(12.0)
                                .color(t.text_muted),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("📋 复制全部文字")
                                            .size(13.0)
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(t.accent_emerald)
                                    .rounding(Rounding::same(7.0)),
                                )
                                .clicked()
                            {
                                self.copy_text(ui.ctx(), plain_text.clone(), "解密文字已复制到剪贴板！");
                            }
                        });
                    });
                    ui.add_space(8.0);

                    theme::well_frame(&t).show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut plain_text.as_str())
                                .desired_rows(6)
                                .desired_width(f32::INFINITY),
                        );
                    });
                }
                DecryptedContent::File {
                    meta,
                    bytes,
                    saved_path,
                } => {
                    let full_filename = if meta.extension.is_empty() {
                        meta.name.clone()
                    } else {
                        format!("{}.{}", meta.name, meta.extension)
                    };

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("✅ 解密成功：文件")
                                .size(15.0)
                                .color(t.accent_emerald)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!("大小: {}", format_file_size(bytes.len())))
                                .size(12.0)
                                .color(t.text_muted),
                        );
                    });
                    ui.add_space(8.0);

                    theme::well_frame(&t).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (icon_rect, _) =
                                ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::hover());
                            let painter = ui.painter_at(icon_rect);
                            painter.rect_filled(icon_rect, Rounding::same(6.0), t.bg_card_elevated);
                            painter.text(
                                icon_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "📁",
                                egui::FontId::new(22.0, egui::FontFamily::Proportional),
                                Color32::WHITE,
                            );

                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(&full_filename)
                                        .size(14.5)
                                        .color(t.text_primary)
                                        .strong(),
                                );
                                if let Some(path) = &saved_path {
                                    ui.label(
                                        egui::RichText::new(format!("✓ 已保存至：{}", path.display()))
                                            .size(11.5)
                                            .color(t.accent_emerald),
                                    );
                                } else {
                                    ui.label(
                                        egui::RichText::new("文件已安全解开在内存中，点击右侧保存到电脑")
                                            .size(11.5)
                                            .color(t.text_muted),
                                    );
                                }
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("💾 另存文件到本地...")
                                                .size(13.0)
                                                .color(Color32::WHITE)
                                                .strong(),
                                        )
                                        .fill(t.accent_emerald)
                                        .min_size(egui::vec2(130.0, 34.0))
                                        .rounding(Rounding::same(8.0)),
                                    )
                                    .clicked()
                                {
                                    if let Some(dest) = rfd::FileDialog::new()
                                        .set_file_name(&full_filename)
                                        .save_file()
                                    {
                                        match fs::write(&dest, &bytes) {
                                            Ok(_) => {
                                                self.notify(
                                                    format!("已保存至：{}", dest.file_name().unwrap_or_default().to_string_lossy()),
                                                    ToastKind::Success,
                                                );
                                                self.decrypted = Some(DecryptedContent::File {
                                                    meta: meta.clone(),
                                                    bytes: bytes.clone(),
                                                    saved_path: Some(dest),
                                                });
                                            }
                                            Err(e) => {
                                                self.notify(format!("保存失败：{e}"), ToastKind::Error);
                                            }
                                        }
                                    }
                                }
                            });
                        });
                    });
                }
            }
        });
    }

    fn do_receive(&mut self) {
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

    // ========================================================
    // TAB 3: 身份与联系人
    // ========================================================
    fn render_identity_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Section 1: My Identity Card
            self.render_my_identity_card(ui);
            ui.add_space(14.0);

            // Section 2: Address book
            self.render_address_book(ui);
        });
    }

    fn render_my_identity_card(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card_frame(&t).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("本机安全身份名片")
                        .size(15.0)
                        .color(t.text_primary)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("(RSA-2048 非对称端对端密钥对)")
                        .size(11.5)
                        .color(t.text_muted),
                );
            });
            ui.add_space(8.0);

            ui.columns(2, |cols| {
                // Left side: information & actions
                cols[0].vertical(|ui| {
                    ui.label(
                        egui::RichText::new("把你的公开身份代码或二维码发给对方，对方即可直接给你发送专属密信。私钥仅保存在本机，任何第三方都无法解开。")
                            .size(12.0)
                            .color(t.text_secondary),
                    );
                    ui.add_space(10.0);

                    // Fingerprint display
                    theme::well_frame(&t).show(ui, |ui| {
                        ui.label(egui::RichText::new("身份指纹 (Fingerprint)：").size(11.5).color(t.text_muted));
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&self.my_fingerprint)
                                    .size(14.0)
                                    .color(t.accent_emerald)
                                    .strong()
                                    .monospace(),
                            );
                            if ui.button(egui::RichText::new("复制指纹").size(11.5)).clicked() {
                                self.copy_text(ui.ctx(), self.my_fingerprint.clone(), "指纹已复制！");
                            }
                        });
                    });
                    ui.add_space(10.0);

                    // Primary Copy Public Key button
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("📋 复制我的公开身份代码")
                                    .size(13.5)
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(t.accent_emerald)
                            .min_size(egui::vec2(180.0, 36.0))
                            .rounding(Rounding::same(8.0)),
                        )
                        .clicked()
                    {
                        self.copy_text(ui.ctx(), self.identity.1.clone(), "公开身份代码已复制！发给好友即可。");
                    }
                    ui.add_space(8.0);

                    // Expandable PEM view
                    ui.collapsing(egui::RichText::new("查看完整公开代码 (PEM)").size(12.0), |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.identity.1.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(4)
                                .desired_width(f32::INFINITY),
                        );
                    });
                    ui.add_space(8.0);

                    // Identity management actions
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(egui::RichText::new("💾 备份私钥到文件").size(11.5)).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("mixin_identity.pem")
                                .save_file()
                            {
                                match fs::write(&path, &self.identity.0) {
                                    Ok(_) => self.notify("私钥已安全备份！请妥善保存该文件。", ToastKind::Success),
                                    Err(e) => self.notify(format!("备份失败：{e}"), ToastKind::Error),
                                }
                            }
                        }

                        if ui.button(egui::RichText::new("📥 导入私钥文件").size(11.5)).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PEM", &["pem", "txt"])
                                .pick_file()
                            {
                                if let Ok(priv_pem) = fs::read_to_string(&path) {
                                    match public_from_private(&priv_pem) {
                                        Ok(pub_pem) => {
                                            self.identity = (priv_pem.clone(), pub_pem);
                                            self.my_fingerprint = compute_fingerprint(&self.identity.1);
                                            save_identity(&self.identity.0);
                                            self.notify("成功导入本机身份！", ToastKind::Success);
                                        }
                                        Err(e) => self.notify(format!("无效的私钥文件：{e}"), ToastKind::Error),
                                    }
                                }
                            }
                        }

                        if ui
                            .button(egui::RichText::new("重置本机身份").size(11.5).color(t.accent_amber))
                            .clicked()
                        {
                            self.show_regenerate_confirm = true;
                        }
                    });

                    if self.show_regenerate_confirm {
                        ui.add_space(6.0);
                        theme::highlighted_card_frame(&t, t.accent_rose).show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("确定要重新生成身份吗？\n重新生成后，旧身份对应的联系人密信将无法解密！")
                                    .size(11.5)
                                    .color(t.accent_rose),
                            );
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("确认重新生成")
                                                .size(11.5)
                                                .color(Color32::WHITE),
                                        )
                                        .fill(t.accent_rose),
                                    )
                                    .clicked()
                                {
                                    if let Ok(new_id) = generate_identity() {
                                        self.identity = new_id;
                                        self.my_fingerprint = compute_fingerprint(&self.identity.1);
                                        save_identity(&self.identity.0);
                                        self.show_regenerate_confirm = false;
                                        self.notify("新身份已生成并保存！", ToastKind::Success);
                                    }
                                }
                                if ui.button(egui::RichText::new("取消").size(11.5)).clicked() {
                                    self.show_regenerate_confirm = false;
                                }
                            });
                        });
                    }
                });

                // Right side: QR Code
                cols[1].vertical_centered(|ui| {
                    ui.label(egui::RichText::new("扫码获取公开身份").size(12.5).color(t.text_secondary));
                    ui.add_space(6.0);
                    theme::render_qr(ui, &self.identity.1, 150.0);
                    ui.add_space(8.0);
                    if ui.button(egui::RichText::new("💾 保存二维码图片 (.png)").size(12.0)).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("mixin_qr.png")
                            .add_filter("PNG 图片", &["png"])
                            .save_file()
                        {
                            match theme::export_qr_png(&self.identity.1, &path) {
                                Ok(_) => self.notify("二维码图片已保存！", ToastKind::Success),
                                Err(e) => self.notify(e, ToastKind::Error),
                            }
                        }
                    }
                });
            });
        });
    }

    fn render_address_book(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card_frame(&t).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("常用联系人簿")
                        .size(15.0)
                        .color(t.text_primary)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(format!("(共 {} 人)", self.contacts.len()))
                        .size(12.0)
                        .color(t.text_muted),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("＋ 添加联系人")
                                    .size(12.5)
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(t.accent_emerald)
                            .rounding(Rounding::same(6.0)),
                        )
                        .clicked()
                    {
                        self.show_add_contact_dialog = !self.show_add_contact_dialog;
                    }
                });
            });
            ui.add_space(6.0);

            // Add contact form
            if self.show_add_contact_dialog {
                theme::highlighted_card_frame(&t, t.accent_emerald).show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("添加新联系人")
                            .size(13.5)
                            .color(t.text_primary)
                            .strong(),
                    );
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("姓名/备注：").size(12.5));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_contact_name)
                                .hint_text("如：张总、李四、家庭Mac…")
                                .desired_width(200.0),
                        );
                    });
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("公开身份代码：").size(12.5));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(egui::RichText::new("📋 粘贴剪贴板").size(11.5)).clicked() {
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    if let Ok(t_str) = cb.get_text() {
                                        self.new_contact_key = t_str;
                                    }
                                }
                            }
                        });
                    });

                    ui.add(
                        egui::TextEdit::multiline(&mut self.new_contact_key)
                            .hint_text("-----BEGIN PUBLIC KEY-----\n...")
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY),
                    );

                    let fp_preview = validate_public_key(&self.new_contact_key);
                    if let Ok(fp) = &fp_preview {
                        ui.label(
                            egui::RichText::new(format!("✓ 公钥指纹: {fp}"))
                                .size(11.5)
                                .color(t.accent_emerald),
                        );
                    }

                    ui.horizontal(|ui| {
                        let can_save = !self.new_contact_name.trim().is_empty() && fp_preview.is_ok();
                        let save_btn = egui::Button::new(
                            egui::RichText::new("保存联系人")
                                .size(12.5)
                                .color(if can_save {
                                    Color32::WHITE
                                } else {
                                    t.text_muted
                                })
                                .strong(),
                        )
                        .fill(if can_save {
                            t.accent_emerald
                        } else {
                            t.bg_card
                        })
                        .rounding(Rounding::same(6.0));

                        if ui.add_enabled(can_save, save_btn).clicked() {
                            match add_contact(
                                &mut self.contacts,
                                &self.new_contact_name,
                                &self.new_contact_key,
                            ) {
                                Ok(c) => {
                                    self.notify(format!("已添加联系人：{}", c.name), ToastKind::Success);
                                    self.new_contact_name.clear();
                                    self.new_contact_key.clear();
                                    self.show_add_contact_dialog = false;
                                }
                                Err(e) => {
                                    self.notify(format!("添加失败：{e}"), ToastKind::Error);
                                }
                            }
                        }

                        if ui.button(egui::RichText::new("取消").size(12.5)).clicked() {
                            self.show_add_contact_dialog = false;
                        }
                    });
                });
                ui.add_space(8.0);
            }

            // Contact list
            if self.contacts.is_empty() {
                ui.label(
                    egui::RichText::new("暂无保存的常用联系人。添加联系人后，可在“发送”页面一键选择。")
                        .size(12.0)
                        .color(t.text_muted),
                );
            } else {
                let mut delete_idx = None;
                let contacts = self.contacts.clone();
                for (i, contact) in contacts.iter().enumerate() {
                    theme::well_frame(&t).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (avatar_rect, _) =
                                ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
                            let painter = ui.painter_at(avatar_rect);
                            painter.rect_filled(avatar_rect, Rounding::same(16.0), t.bg_card_elevated);
                            let initial = contact.name.chars().next().unwrap_or('?').to_string();
                            painter.text(
                                avatar_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                initial,
                                egui::FontId::new(14.0, egui::FontFamily::Proportional),
                                t.text_primary,
                            );

                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(&contact.name)
                                        .size(14.0)
                                        .color(t.text_primary)
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("指纹: {}", contact.fingerprint))
                                        .size(11.5)
                                        .color(t.text_muted),
                                );
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .button(egui::RichText::new("🗑").size(12.0).color(t.accent_rose))
                                    .clicked()
                                {
                                    delete_idx = Some(i);
                                }

                                if ui.button(egui::RichText::new("📋 复制公钥").size(11.5)).clicked() {
                                    self.copy_text(ui.ctx(), contact.public_key.clone(), "联系人公钥已复制！");
                                }

                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("✉️ 发密信给TA")
                                                .size(12.0)
                                                .color(Color32::WHITE),
                                        )
                                        .fill(t.accent_emerald)
                                        .rounding(Rounding::same(6.0)),
                                    )
                                    .clicked()
                                {
                                    self.tab = Tab::Send;
                                    self.send_mode = SendMode::Contact;
                                    self.selected_contact_idx = Some(i);
                                    self.notify(format!("已选择发信给：{}", contact.name), ToastKind::Success);
                                }
                            });
                        });
                    });
                    ui.add_space(4.0);
                }

                if let Some(i) = delete_idx {
                    remove_contact(&mut self.contacts, i);
                    self.notify("联系人已删除", ToastKind::Info);
                }
            }
        });
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
                        self.notify("已载入拖入的密信文件", ToastKind::Success);
                    }
                } else {
                    self.tab = Tab::Send;
                    self.load_file_for_send(path);
                }
            }
        }

        egui::TopBottomPanel::top("top_header")
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                self.render_header(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_toast(ui);

            match self.tab {
                Tab::Send => self.render_send_tab(ui),
                Tab::Receive => self.render_receive_tab(ui),
                Tab::Identity => self.render_identity_tab(ui),
            }
        });

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
        .unwrap_or(true)
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

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([840.0, 720.0])
            .with_min_inner_size([740.0, 580.0])
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
        let decoded = base64::engine::general_purpose::STANDARD.decode(dec_envelope.data).unwrap();
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
        let plain = base64::engine::general_purpose::STANDARD.decode(dec_env.data).unwrap();
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
        let restored_bytes = base64::engine::general_purpose::STANDARD.decode(dec_env.data).unwrap();
        assert_eq!(&restored_bytes, file_data);

        // Decrypt with wrong password must fail
        assert!(decrypt_payload("wrong_password", &cipher).is_err());
    }
}
