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

    // --- UI 渲染主入口 ---
    fn render_ui(&mut self, ctx: &egui::Context) {
        let t = self.theme();
        egui::SidePanel::left("navigation")
            .exact_width(184.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(t.bg_header)
                    .inner_margin(Margin::symmetric(18.0, 12.0)),
            )
            .show(ctx, |ui| self.render_header(ui));

        egui::TopBottomPanel::bottom("feedback")
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(t.bg_canvas)
                    .inner_margin(Margin::symmetric(24.0, 8.0)),
            )
            .show(ctx, |ui| {
                if self.toast.is_some() {
                    self.render_toast(ui);
                } else {
                    ui.label(
                        egui::RichText::new("密信仅负责加密与解密，请自行将生成的密信发送给对方。")
                            .size(11.0)
                            .color(t.text_muted),
                    );
                }
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(t.bg_canvas)
                    .inner_margin(Margin::symmetric(28.0, 28.0)),
            )
            .show(ctx, |ui| {
                let width = ui.available_width().min(720.0);
                let inset = ((ui.available_width() - width) / 2.0).max(0.0);
                ui.horizontal_top(|ui| {
                    ui.add_space(inset);
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        let (title, description) = match self.tab {
                            Tab::Send => ("发送密信", "把文字或文件加密，交给你信任的人。"),
                            Tab::Receive => ("接收密信", "导入收到的密信，在本机解开。"),
                            Tab::Identity => (
                                "身份与联系人",
                                "分享公开身份，让常用联系人无需口令给你发密信。",
                            ),
                        };
                        ui.label(egui::RichText::new(title).size(26.0).strong());
                        ui.label(
                            egui::RichText::new(description)
                                .size(13.0)
                                .color(t.text_secondary),
                        );
                        ui.add_space(24.0);
                        match self.tab {
                            Tab::Send => self.render_send_tab(ui),
                            Tab::Receive => self.render_receive_tab(ui),
                            Tab::Identity => self.render_identity_tab(ui),
                        }
                    });
                });
            });
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, Rounding::same(9.0), t.accent_emerald);
            let envelope = rect.shrink(8.0);
            painter.rect_stroke(envelope, 2.0, Stroke::new(1.4, t.text_on_accent));
            painter.line_segment(
                [envelope.left_top(), envelope.center()],
                Stroke::new(1.4, t.text_on_accent),
            );
            painter.line_segment(
                [envelope.right_top(), envelope.center()],
                Stroke::new(1.4, t.text_on_accent),
            );
            ui.label(egui::RichText::new("密信").size(22.0).strong());
        });
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("让秘密，安心抵达")
                .size(12.0)
                .color(t.text_secondary),
        );
        ui.add_space(36.0);
        self.render_tabs(ui);
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.add_space(20.0);
            let label = if self.dark_mode {
                "切换浅色外观"
            } else {
                "切换深色外观"
            };
            if ui
                .add_sized(
                    [ui.available_width(), 32.0],
                    egui::Button::new(label).frame(false),
                )
                .clicked()
            {
                self.dark_mode = !self.dark_mode;
                save_theme_preference(self.dark_mode);
                theme::apply_theme(ui.ctx(), self.dark_mode);
            }
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("在本机加密与解密")
                    .size(11.5)
                    .color(t.text_secondary),
            );
            ui.label(
                egui::RichText::new("文字与文件 · 端对端保护")
                    .size(11.0)
                    .color(t.text_muted),
            );
        });
    }

    fn render_tabs(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        for (tab, title, detail) in [
            (Tab::Send, "发送密信", "加密文字或文件"),
            (Tab::Receive, "接收密信", "解开收到的内容"),
            (Tab::Identity, "身份与联系人", "名片与常用联系人"),
        ] {
            let selected = self.tab == tab;
            let response = ui.add_sized(
                [ui.available_width(), 40.0],
                egui::Button::new(egui::RichText::new(title).size(14.0).color(if selected {
                    t.accent_emerald
                } else {
                    t.text_secondary
                }))
                .fill(if selected {
                    t.bg_selection
                } else {
                    Color32::TRANSPARENT
                })
                .stroke(Stroke::NONE)
                .rounding(8.0),
            );
            theme::focus_outline(ui, &response, &t);
            if response.hovered() {
                ui.painter()
                    .rect_stroke(response.rect, 8.0, Stroke::new(1.0, t.border_card_light));
            }
            if response.on_hover_text(detail).clicked() {
                self.tab = tab;
            }
            ui.add_space(4.0);
        }
    }

    fn render_toast(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        let Some(toast) = self.toast.clone() else {
            return;
        };
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
        if !self.generated_cipher.is_empty() {
            egui::ScrollArea::vertical()
                .id_salt("generated_scroll")
                .show(ui, |ui| {
                    self.render_generated_output(ui);
                });
            return;
        }
        let body_height = (ui.available_height() - 84.0).max(120.0);
        egui::ScrollArea::vertical()
            .id_salt("send_scroll")
            .max_height(body_height)
            .show(ui, |ui| {
                self.render_payload_input(ui);
                ui.add_space(16.0);
                theme::section_heading(ui, "02", "设置接收方式", &t);
                self.render_send_mode_selector(ui);
                ui.add_space(12.0);
                if self.send_mode == SendMode::Contact {
                    self.render_recipient_selector(ui);
                } else {
                    self.render_password_input(ui);
                }
            });
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);
        let hint = self.send_blocker();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(hint.is_none(), theme::primary_button("生成密信", &t))
                .clicked()
            {
                self.do_send();
            }
            ui.label(
                egui::RichText::new(hint.unwrap_or("生成后，可复制文本或保存为文件分享"))
                    .size(12.0)
                    .color(t.text_secondary),
            );
        });
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

    fn render_send_mode_selector(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::segmented_control(
            ui,
            &mut self.send_mode,
            &[
                (SendMode::Password, "使用口令"),
                (SendMode::Contact, "发给联系人"),
            ],
            &t,
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(match self.send_mode {
                SendMode::Password => "对方输入你设置的口令，即可解开。适合临时传递。",
                SendMode::Contact => "使用对方的公开身份加密，只有对应身份能解开，无需口令。",
            })
            .size(12.0)
            .color(t.text_secondary),
        );
    }

    fn render_recipient_selector(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card_frame(&t).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("选择接收人")
                        .size(14.0)
                        .color(t.text_primary)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(egui::RichText::new("添加联系人").size(12.0))
                        .clicked()
                    {
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
                        let chip_text = contact.name.clone();
                        let chip =
                            egui::Button::new(egui::RichText::new(chip_text).size(12.5).color(
                                if is_selected {
                                    t.text_on_accent
                                } else {
                                    t.text_primary
                                },
                            ))
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
                        egui::RichText::new("使用公开身份代码")
                            .size(12.5)
                            .color(if is_custom {
                                t.accent_emerald
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
                        egui::RichText::new("对方的公开身份代码")
                            .size(12.5)
                            .color(t.text_secondary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(egui::RichText::new("粘贴公开身份").size(12.0))
                            .clicked()
                        {
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
                            ui.label(
                                egui::RichText::new(format!("{e}"))
                                    .size(12.0)
                                    .color(t.accent_rose),
                            );
                        }
                    }
                }
            }
        });
    }

    fn render_payload_input(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::section_heading(ui, "01", "准备内容", &t);
        ui.horizontal_wrapped(|ui| {
            let mut kind = self.payload_type;
            theme::segmented_control(
                ui,
                &mut kind,
                &[(PayloadType::Text, "文字"), (PayloadType::File, "文件")],
                &t,
            );
            if kind == PayloadType::Text && self.payload_type != kind {
                self.send_file = None;
                self.payload_type = kind;
            } else if kind == PayloadType::File {
                self.payload_type = kind;
            }
            if self.payload_type == PayloadType::Text {
                if ui.button("粘贴").clicked() {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        if let Ok(text) = cb.get_text() {
                            self.send_text.push_str(&text);
                        }
                    }
                }
                if !self.send_text.is_empty() && ui.button("清空").clicked() {
                    self.send_text.clear();
                }
            }
        });
        ui.add_space(10.0);
        if self.payload_type == PayloadType::File {
            theme::well_frame(&t).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.add_space(8.0);
                if let Some(file) = &self.send_file {
                    let name = if file.extension.is_empty() {
                        file.name.clone()
                    } else {
                        format!("{}.{}", file.name, file.extension)
                    };
                    ui.label(egui::RichText::new(name).size(15.0).strong());
                    ui.label(
                        egui::RichText::new(format_file_size(file.size)).color(t.text_secondary),
                    );
                } else {
                    ui.label(egui::RichText::new("将文件拖到窗口，或点击选择").size(15.0));
                    ui.label(
                        egui::RichText::new("支持文档、图片、压缩包等任意文件")
                            .size(12.0)
                            .color(t.text_secondary),
                    );
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(if self.send_file.is_some() {
                            "更换文件"
                        } else {
                            "选择文件"
                        })
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.load_file_for_send(path);
                        }
                    }
                    if self.send_file.is_some() && ui.button("移除").clicked() {
                        self.send_file = None;
                    }
                });
                ui.add_space(8.0);
            });
        } else {
            ui.add(
                egui::TextEdit::multiline(&mut self.send_text)
                    .hint_text("写下或粘贴需要保密的内容…")
                    .desired_rows(4)
                    .margin(egui::vec2(14.0, 12.0))
                    .desired_width(f32::INFINITY),
            );
            ui.label(
                egui::RichText::new(format!("{} 字", self.send_text.chars().count()))
                    .size(11.5)
                    .color(t.text_muted),
            );
        }
    }

    fn render_password_input(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        ui.columns(2, |cols| {
            cols[0].label(
                egui::RichText::new("解密口令")
                    .size(12.0)
                    .color(t.text_secondary),
            );
            cols[0].add(
                egui::TextEdit::singleline(&mut self.send_password)
                    .password(!self.send_show_password)
                    .hint_text("设置口令")
                    .desired_width(f32::INFINITY),
            );
            cols[1].label(
                egui::RichText::new("确认口令")
                    .size(12.0)
                    .color(t.text_secondary),
            );
            cols[1].add(
                egui::TextEdit::singleline(&mut self.send_confirm)
                    .password(!self.send_show_password)
                    .hint_text("再次输入")
                    .desired_width(f32::INFINITY),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.send_show_password, "显示口令");
            if ui
                .button("随机生成并复制")
                .on_hover_text("生成随机口令，并同时填入确认栏")
                .clicked()
            {
                let pwd = generate_memorable_password();
                self.send_password = pwd.clone();
                self.send_confirm = pwd.clone();
                self.copy_text(ui.ctx(), pwd, "口令已生成并复制，请通过另一渠道告知对方");
            }
            if !self.send_confirm.is_empty() {
                let matches = self.send_password == self.send_confirm;
                ui.label(
                    egui::RichText::new(if matches {
                        "口令一致"
                    } else {
                        "两次口令不一致"
                    })
                    .size(12.0)
                    .color(if matches {
                        t.accent_emerald
                    } else {
                        t.accent_rose
                    }),
                );
            }
        });
    }

    fn render_generated_output(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::highlighted_card_frame(&t, t.accent_emerald).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new("密信已生成")
                        .size(15.0)
                        .color(t.accent_emerald)
                        .strong(),
                );
                if let Some(summary) = &self.generated_summary {
                    ui.label(egui::RichText::new(summary).size(12.0).color(t.text_muted));
                }
            });
            ui.add_space(6.0);

            ui.label(
                egui::RichText::new(
                    "复制密信文本，或保存为 .mixin 文件，再通过你常用的聊天工具发送给对方。",
                )
                .color(t.text_secondary),
            );
            ui.add_space(16.0);
            // Dual action buttons
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("复制密信文本")
                                .size(13.5)
                                .color(t.text_on_accent)
                                .strong(),
                        )
                        .fill(t.accent_emerald)
                        .min_size(egui::vec2(150.0, 34.0))
                        .rounding(Rounding::same(8.0)),
                    )
                    .clicked()
                {
                    self.copy_text(
                        ui.ctx(),
                        self.generated_cipher.clone(),
                        "密信文本已复制到剪贴板！可直接发送给对方。",
                    );
                }

                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("保存密信文件")
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
                                    format!(
                                        "已保存至：{}",
                                        path.file_name().unwrap_or_default().to_string_lossy()
                                    ),
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

            ui.add_space(16.0);
            if self.send_mode == SendMode::Password {
                ui.label(
                    egui::RichText::new(
                        "请分开传递密信与口令，例如用聊天工具发密信，电话告知口令。",
                    )
                    .size(12.0)
                    .color(t.text_secondary),
                );
            } else {
                ui.label(
                    egui::RichText::new("对方需使用对应的本机身份，在“接收密信”中解密。")
                        .size(12.0)
                        .color(t.text_secondary),
                );
            }
            ui.add_space(16.0);
            ui.collapsing("查看加密文本", |ui| {
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.generated_cipher.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY),
                        );
                    });
            });
            ui.add_space(16.0);
            if ui.button("返回编辑").clicked() {
                self.generated_cipher.clear();
                self.generated_summary = None;
            }
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

    // ========================================================
    // TAB 2: 接收解密
    // ========================================================
    fn render_receive_tab(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        egui::ScrollArea::vertical().id_salt("receive_scroll").show(ui, |ui| {
            // Input card
            theme::card_frame(&t).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("01  导入密信")
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

                        if ui.button(egui::RichText::new("打开文件").size(12.0)).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("密信文件", &["mixin", "txt"])
                                .pick_file()
                            {
                                if let Ok(content) = fs::read_to_string(&path) {
                                    self.receive_input = content.trim().to_string();
                                    self.decrypted = None;
                                    self.notify("密信文件已载入", ToastKind::Success);
                                }
                            }
                        }

                        if ui.button(egui::RichText::new("粘贴密信").size(12.0)).clicked() {
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                if let Ok(t_str) = cb.get_text() {
                                    self.receive_input = t_str.trim().to_string();
                                    self.decrypted = None;
                                    self.notify("已从剪贴板粘贴", ToastKind::Success);
                                }
                            }
                        }
                    });
                });
                ui.add_space(8.0);

                let input_response = ui.add(
                    egui::TextEdit::multiline(&mut self.receive_input)
                        .hint_text("在此粘贴完整密信文本，或拖入 .mixin 文件…")
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(5)
                        .desired_width(f32::INFINITY),
                );
                if input_response.changed() { self.decrypted = None; }
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
                            egui::RichText::new("无法识别这段内容")
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
        ui.add_space(20.0);
        ui.label(egui::RichText::new("等待一封密信").size(18.0).strong());
        ui.label(egui::RichText::new("粘贴密信文本，或将收到的 .mixin 文件拖到窗口。\n密信会自动识别接收方式，再提示你解密。").color(t.text_secondary));
    }

    fn render_identity_decrypt_card(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card_frame(&t).show(ui, |ui| {
            ui.horizontal(|ui| {
                theme::pill_badge(ui, "专属密信", t.text_secondary);
                ui.label(
                    egui::RichText::new("这是一封专属密信")
                        .size(13.0)
                        .color(t.text_primary),
                );
            });
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("无需口令。请使用接收方对应的本机身份解密。")
                    .size(12.0)
                    .color(t.text_muted),
            );
            ui.add_space(8.0);

            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("使用本机身份解密")
                            .size(14.0)
                            .color(t.text_on_accent)
                            .strong(),
                    )
                    .fill(t.accent_emerald)
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
        theme::card_frame(&t).show(ui, |ui| {
            ui.horizontal(|ui| {
                theme::pill_badge(ui, "口令密信", t.text_secondary);
                ui.label(
                    egui::RichText::new("输入发送方告知的口令")
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

                ui.checkbox(
                    &mut self.receive_show_password,
                    egui::RichText::new("显示").size(12.0),
                );

                let can_decrypt = !self.receive_password.is_empty();
                let btn = egui::Button::new(
                    egui::RichText::new("解密密信")
                        .size(13.5)
                        .color(if can_decrypt {
                            t.text_on_accent
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
        let Some(content) = self.decrypted.clone() else {
            return;
        };

        theme::highlighted_card_frame(&t, t.accent_emerald).show(ui, |ui| match content {
            DecryptedContent::Text(plain_text) => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("已解密 · 文字")
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
                                    egui::RichText::new("复制文字")
                                        .size(13.0)
                                        .color(t.text_on_accent)
                                        .strong(),
                                )
                                .fill(t.accent_emerald)
                                .rounding(Rounding::same(7.0)),
                            )
                            .clicked()
                        {
                            self.copy_text(
                                ui.ctx(),
                                plain_text.clone(),
                                "解密文字已复制到剪贴板！",
                            );
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

                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("已解密 · 文件")
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
                    ui.horizontal_wrapped(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::hover());
                        let painter = ui.painter_at(icon_rect);
                        painter.rect_filled(icon_rect, Rounding::same(6.0), t.bg_card_elevated);
                        painter.text(
                            icon_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "文",
                            egui::FontId::new(22.0, egui::FontFamily::Proportional),
                            t.text_on_accent,
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
                                    egui::RichText::new(
                                        "文件已安全解开在内存中，点击右侧保存到电脑",
                                    )
                                    .size(11.5)
                                    .color(t.text_muted),
                                );
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("保存文件")
                                            .size(13.0)
                                            .color(t.text_on_accent)
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
                                                format!(
                                                    "已保存至：{}",
                                                    dest.file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy()
                                                ),
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
        });
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

    // ========================================================
    // TAB 3: 身份与联系人
    // ========================================================
    fn render_identity_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("identity_scroll")
            .show(ui, |ui| {
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
                    egui::RichText::new("我的公开身份")
                        .size(15.0)
                        .color(t.text_primary)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("可分享给联系人")
                        .size(11.5)
                        .color(t.text_muted),
                );
            });
            ui.add_space(8.0);

            ui.columns(2, |cols| {
                // Left side: information & actions
                cols[0].vertical(|ui| {
                    ui.label(
                        egui::RichText::new("把公开身份发给对方，对方就能给你发送无需口令的专属密信。")
                            .size(12.0)
                            .color(t.text_secondary),
                    );
                    ui.add_space(10.0);

                    // Fingerprint display
                    theme::well_frame(&t).show(ui, |ui| {
                        ui.label(egui::RichText::new("身份指纹").size(11.5).color(t.text_muted));
                        ui.vertical(|ui| {
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
                                egui::RichText::new("复制公开身份")
                                    .size(13.5)
                                    .color(t.text_on_accent)
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
                    ui.collapsing(egui::RichText::new("查看公开身份代码").size(12.0), |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.identity.1.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(4)
                                .desired_width(f32::INFINITY),
                        );
                    });
                    ui.add_space(8.0);

                    // Low-frequency identity operations use progressive disclosure.
                    ui.collapsing("身份备份与恢复", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(egui::RichText::new("备份私钥").size(11.5)).clicked() {
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

                        if ui.button(egui::RichText::new("导入私钥").size(11.5)).clicked() {
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
                                                .color(t.text_on_accent),
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
                });

                // Right side: QR Code
                cols[1].vertical_centered(|ui| {
                    ui.label(egui::RichText::new("扫码获取公开身份").size(12.5).color(t.text_secondary));
                    ui.add_space(6.0);
                    theme::render_qr(ui, &self.identity.1, 150.0);
                    ui.add_space(8.0);
                    if ui.button(egui::RichText::new("保存二维码").size(12.0)).clicked() {
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
                    egui::RichText::new("常用联系人")
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
                                    .color(t.text_on_accent)
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
                            if ui.button(egui::RichText::new("粘贴").size(11.5)).clicked() {
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
                        let can_save =
                            !self.new_contact_name.trim().is_empty() && fp_preview.is_ok();
                        let save_btn = egui::Button::new(
                            egui::RichText::new("保存联系人")
                                .size(12.5)
                                .color(if can_save {
                                    t.text_on_accent
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
                                    self.notify(
                                        format!("已添加联系人：{}", c.name),
                                        ToastKind::Success,
                                    );
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
                    egui::RichText::new(
                        "暂无保存的常用联系人。添加联系人后，可在“发送”页面一键选择。",
                    )
                    .size(12.0)
                    .color(t.text_muted),
                );
            } else {
                let mut delete_idx = None;
                let contacts = self.contacts.clone();
                for (i, contact) in contacts.iter().enumerate() {
                    theme::well_frame(&t).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (avatar_rect, _) = ui
                                .allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
                            let painter = ui.painter_at(avatar_rect);
                            painter.rect_filled(
                                avatar_rect,
                                Rounding::same(16.0),
                                t.bg_card_elevated,
                            );
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
                        });
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("发送密信")
                                            .size(12.0)
                                            .color(t.text_on_accent),
                                    )
                                    .fill(t.accent_emerald)
                                    .rounding(Rounding::same(6.0)),
                                )
                                .clicked()
                            {
                                self.tab = Tab::Send;
                                self.generated_cipher.clear();
                                self.generated_summary = None;
                                self.send_mode = SendMode::Contact;
                                self.selected_contact_idx = Some(i);
                                self.notify(
                                    format!("已选择发信给：{}", contact.name),
                                    ToastKind::Success,
                                );
                            }
                            if ui
                                .button(egui::RichText::new("复制公开身份").size(11.5))
                                .clicked()
                            {
                                self.copy_text(
                                    ui.ctx(),
                                    contact.public_key.clone(),
                                    "联系人公钥已复制！",
                                );
                            }
                            if ui
                                .button(egui::RichText::new("删除").size(12.0).color(t.accent_rose))
                                .clicked()
                            {
                                delete_idx = Some(i);
                            }
                        });
                    });
                    ui.add_space(4.0);
                }

                if let Some(i) = delete_idx {
                    remove_contact(&mut self.contacts, i);
                    self.selected_contact_idx = self.selected_contact_idx.and_then(|selected| {
                        if selected == i {
                            None
                        } else {
                            Some(if selected > i { selected - 1 } else { selected })
                        }
                    });
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

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 800.0])
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
