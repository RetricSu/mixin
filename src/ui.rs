//! Window layout and views. Logic (encrypt, decrypt, validation) lives in `main.rs`.
use super::*;
use crate::theme::{self, Icon};
use eframe::egui::{Align, Align2, Color32, Layout, Margin, RichText, Stroke};

const SIDEBAR_WIDTH: f32 = 176.0;
const CONTENT_MAX_WIDTH: f32 = 640.0;
const FOOTER_HEIGHT: f32 = 56.0;
const EDITOR_MIN_HEIGHT: f32 = 112.0;

impl App {
    // ========================================================
    // Shell
    // ========================================================
    pub(crate) fn render_ui(&mut self, ctx: &egui::Context) {
        let t = self.theme();
        egui::SidePanel::left("navigation")
            .exact_width(SIDEBAR_WIDTH)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(t.bg_header)
                    .stroke(Stroke::new(1.0, t.border_card))
                    .inner_margin(Margin {
                        left: 12.0,
                        right: 12.0,
                        top: 18.0,
                        bottom: 14.0,
                    }),
            )
            .show(ctx, |ui| self.render_sidebar(ui));

        let central = egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(t.bg_canvas)
                    .inner_margin(Margin {
                        left: 24.0,
                        right: 24.0,
                        top: 14.0,
                        bottom: 12.0,
                    }),
            )
            .show(ctx, |ui| {
                let width = ui.available_width().min(CONTENT_MAX_WIDTH);
                let inset = ((ui.available_width() - width) / 2.0).max(0.0);
                ui.horizontal_top(|ui| {
                    ui.add_space(inset);
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        self.render_page_title(ui);
                        ui.add_space(10.0);
                        match self.tab {
                            Tab::Send => self.render_send_tab(ui),
                            Tab::Receive => self.render_receive_tab(ui),
                            Tab::Identity => self.render_identity_tab(ui),
                        }
                    });
                });
            });

        self.render_toast(ctx, central.response.rect);
        self.render_drop_overlay(ctx);
    }

    fn render_page_title(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        let title = match self.tab {
            Tab::Send => "发送密信",
            Tab::Receive => "接收密信",
            Tab::Identity => "身份与联系人",
        };
        ui.horizontal(|ui| {
            ui.set_min_height(30.0);
            ui.label(RichText::new(title).size(20.0).strong().color(t.text_primary));
            if self.tab == Tab::Send && !self.generated_cipher.is_empty() {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme::ghost_button(ui, "返回编辑", &t).clicked() {
                        self.generated_cipher.clear();
                        self.generated_summary = None;
                    }
                });
            }
        });
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(rect, 7.0, t.accent_emerald);
            let envelope = rect.shrink2(egui::vec2(6.5, 8.5));
            let stroke = Stroke::new(1.4, Color32::WHITE);
            painter.rect_stroke(envelope, 1.5, stroke);
            painter.line_segment([envelope.left_top(), envelope.center() + egui::vec2(0.0, 1.0)], stroke);
            painter.line_segment([envelope.right_top(), envelope.center() + egui::vec2(0.0, 1.0)], stroke);
            ui.add_space(2.0);
            ui.label(RichText::new("密信").size(17.0).strong().color(t.text_primary));
        });
        ui.add_space(22.0);

        ui.spacing_mut().item_spacing.y = 2.0;
        for (tab, icon, title) in [
            (Tab::Send, Icon::Send, "发送密信"),
            (Tab::Receive, Icon::Receive, "接收密信"),
            (Tab::Identity, Icon::Identity, "身份与联系人"),
        ] {
            if theme::nav_item(ui, icon, title, self.tab == tab, &t).clicked() {
                self.tab = tab;
            }
        }

        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let mut dark = self.dark_mode;
            ui.horizontal(|ui| {
                let total = ui.available_width();
                ui.spacing_mut().item_spacing.x = 0.0;
                egui::Frame::none()
                    .fill(t.bg_card_elevated)
                    .rounding(theme::RADIUS + 1.0)
                    .inner_margin(Margin::same(2.0))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        let half = (total - 6.0) / 2.0;
                        for (value, icon, label) in
                            [(false, Icon::Sun, "浅色"), (true, Icon::Moon, "深色")]
                        {
                            let selected = dark == value;
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(half, 26.0),
                                egui::Sense::click(),
                            );
                            let fill = if selected {
                                t.bg_card
                            } else if response.hovered() {
                                t.bg_card_hover
                            } else {
                                Color32::TRANSPARENT
                            };
                            ui.painter().rect(
                                rect,
                                theme::RADIUS - 1.0,
                                fill,
                                if selected {
                                    Stroke::new(1.0, t.border_card)
                                } else {
                                    Stroke::NONE
                                },
                            );
                            let color = if selected { t.text_primary } else { t.text_muted };
                            let galley = ui.painter().layout_no_wrap(
                                label.to_owned(),
                                egui::FontId::proportional(12.0),
                                color,
                            );
                            let content_w = 13.0 + 5.0 + galley.size().x;
                            let x = rect.center().x - content_w / 2.0;
                            theme::paint_icon(
                                ui.painter(),
                                egui::Rect::from_center_size(
                                    egui::pos2(x + 6.5, rect.center().y),
                                    egui::vec2(13.0, 13.0),
                                ),
                                icon,
                                color,
                            );
                            ui.painter().galley(
                                egui::pos2(x + 18.0, rect.center().y - galley.size().y / 2.0),
                                galley,
                                color,
                            );
                            theme::focus_outline(ui, &response, &t);
                            if response
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                dark = value;
                            }
                        }
                    });
            });
            if dark != self.dark_mode {
                self.dark_mode = dark;
                save_theme_preference(dark);
                theme::apply_theme(ui.ctx(), dark);
            }
        });
    }

    fn render_toast(&mut self, ctx: &egui::Context, content_rect: egui::Rect) {
        let t = self.theme();
        let Some(toast) = self.toast.clone() else {
            return;
        };
        let elapsed = toast.created_at.elapsed().as_secs_f32();
        if elapsed > toast.duration_secs {
            self.toast = None;
            return;
        }
        let opacity = ((toast.duration_secs - elapsed) / 0.3).clamp(0.0, 1.0)
            * (elapsed / 0.15).clamp(0.0, 1.0);
        let (icon, color) = match toast.kind {
            ToastKind::Success => (Icon::Check, t.accent_emerald),
            ToastKind::Info => (Icon::Info, t.accent_sky),
            ToastKind::Warning => (Icon::Warning, t.accent_amber),
            ToastKind::Error => (Icon::Error, t.accent_rose),
        };
        let max_width = (content_rect.width() - 48.0).clamp(200.0, 520.0);
        egui::Area::new(egui::Id::new("toast"))
            .order(egui::Order::Foreground)
            .pivot(Align2::CENTER_BOTTOM)
            .fixed_pos(egui::pos2(
                content_rect.center().x,
                content_rect.bottom() - 18.0 + (1.0 - opacity) * 6.0,
            ))
            .show(ctx, |ui| {
                ui.multiply_opacity(opacity);
                egui::Frame::none()
                    .fill(t.bg_card)
                    .stroke(Stroke::new(1.0, t.border_card_light))
                    .rounding(10.0)
                    .inner_margin(Margin {
                        left: 12.0,
                        right: 6.0,
                        top: 6.0,
                        bottom: 6.0,
                    })
                    .shadow(egui::epaint::Shadow {
                        offset: egui::vec2(0.0, 6.0),
                        blur: 20.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(if self.dark_mode { 110 } else { 28 }),
                    })
                    .show(ui, |ui| {
                        ui.set_max_width(max_width);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            theme::icon(ui, icon, 16.0, color);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&toast.message).size(13.0).color(t.text_primary),
                                )
                                .wrap(),
                            );
                            let (rect, response) = ui
                                .allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                            if response.hovered() {
                                ui.painter().rect_filled(rect, 5.0, t.bg_card_hover);
                            }
                            theme::paint_icon(
                                ui.painter(),
                                egui::Rect::from_center_size(rect.center(), egui::vec2(12.0, 12.0)),
                                Icon::Close,
                                t.text_muted,
                            );
                            if response.on_hover_text("关闭").clicked() {
                                self.toast = None;
                            }
                        });
                    });
            });
    }

    fn render_drop_overlay(&mut self, ctx: &egui::Context) {
        let hovered = ctx.input(|i| i.raw.hovered_files.clone());
        if hovered.is_empty() {
            return;
        }
        let t = self.theme();
        let is_mixin = hovered.iter().any(|f| {
            f.path
                .as_ref()
                .is_some_and(|p| p.extension().is_some_and(|ext| ext == "mixin"))
        });
        let screen = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("drop_overlay"),
        ));
        let bg = t.bg_canvas;
        painter.rect_filled(
            screen,
            0.0,
            Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), 235),
        );
        let zone = screen.shrink(18.0);
        painter.rect_stroke(zone, 14.0, Stroke::new(1.5, t.border_accent));
        let center = zone.center();
        theme::paint_icon(
            &painter,
            egui::Rect::from_center_size(center - egui::vec2(0.0, 30.0), egui::vec2(32.0, 32.0)),
            if is_mixin { Icon::Receive } else { Icon::Upload },
            t.text_accent,
        );
        painter.text(
            center + egui::vec2(0.0, 6.0),
            Align2::CENTER_CENTER,
            if is_mixin { "松开以打开密信" } else { "松开以加密这个文件" },
            egui::FontId::proportional(16.0),
            t.text_primary,
        );
        painter.text(
            center + egui::vec2(0.0, 30.0),
            Align2::CENTER_CENTER,
            if is_mixin {
                "将切换到接收密信"
            } else {
                "文件只在本机读取和加密"
            },
            egui::FontId::proportional(12.5),
            t.text_muted,
        );
    }

    // ========================================================
    // Send
    // ========================================================
    fn render_send_tab(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        if !self.generated_cipher.is_empty() {
            egui::ScrollArea::vertical()
                .id_salt("generated_scroll")
                .show(ui, |ui| self.render_generated_output(ui));
            return;
        }
        let body_height = (ui.available_height() - FOOTER_HEIGHT).max(120.0);
        egui::ScrollArea::vertical()
            .id_salt("send_scroll")
            .max_height(body_height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let mut kind = self.payload_type;
                theme::section_header(ui, "内容", &t, |ui| {
                    theme::segmented_control(
                        ui,
                        &mut kind,
                        &[(PayloadType::Text, "文字"), (PayloadType::File, "文件")],
                        &t,
                    );
                });
                if kind != self.payload_type {
                    if kind == PayloadType::Text {
                        self.send_file = None;
                    }
                    self.payload_type = kind;
                }
                self.render_payload_input(ui);
                ui.add_space(10.0);

                let mut mode = self.send_mode;
                theme::section_header(ui, "接收方式", &t, |ui| {
                    theme::segmented_control(
                        ui,
                        &mut mode,
                        &[
                            (SendMode::Password, "使用口令"),
                            (SendMode::Contact, "发给联系人"),
                        ],
                        &t,
                    );
                });
                self.send_mode = mode;
                ui.add_space(2.0);
                if self.send_mode == SendMode::Contact {
                    self.render_recipient_selector(ui);
                } else {
                    self.render_password_input(ui);
                }
                ui.add_space(10.0);
            });

        // Pinned footer: what is missing on the left, the one primary action on the right.
        let hint = self.send_blocker();
        theme::hairline(ui, &t);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.set_min_height(theme::PRIMARY_HEIGHT);
            if let Some(hint) = hint {
                theme::icon(ui, Icon::Info, 14.0, t.text_muted);
                ui.add(
                    egui::Label::new(RichText::new(hint).size(12.5).color(t.text_secondary))
                        .truncate(),
                );
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if primary_action(ui, "生成密信", hint.is_none(), &t).clicked() {
                    self.do_send();
                }
            });
        });
    }

    fn render_payload_input(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::paper_frame(&t).inner_margin(Margin::symmetric(14.0, 10.0)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if self.payload_type == PayloadType::File {
                self.render_file_picker(ui);
                return;
            }
            ui.add(
                egui::TextEdit::multiline(&mut self.send_text)
                    .hint_text("写下或粘贴需要保密的内容…")
                    .frame(false)
                    .desired_rows(2)
                    .min_size(egui::vec2(0.0, 48.0))
                    .margin(egui::vec2(0.0, 2.0))
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(2.0);
            if !self.send_text.is_empty() {
                ui.horizontal(|ui| {
                    if theme::ghost_button(ui, "清空", &t).clicked() {
                        self.send_text.clear();
                    }
                });
            }
        });
    }

    fn render_file_picker(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        if let Some(file) = &self.send_file {
            let name = display_name(&file.name, &file.extension);
            let size = format_file_size(file.size);
            let extension = file.extension.clone();
            let mut pick = false;
            let mut remove = false;
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), EDITOR_MIN_HEIGHT),
                Layout::left_to_right(Align::Center),
                |ui| {
                    file_tile(ui, &extension, &t);
                    ui.add_space(4.0);
                    let buttons_width = 150.0;
                    ui.allocate_ui_with_layout(
                        egui::vec2((ui.available_width() - buttons_width).max(80.0), 44.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.add(egui::Label::new(RichText::new(&name).strong()).truncate())
                                .on_hover_text(&name);
                            theme::caption(ui, size, &t);
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        remove = theme::ghost_button(ui, "移除", &t).clicked();
                        pick = theme::secondary_button(ui, "更换文件", &t).clicked();
                    });
                },
            );
            if remove {
                self.send_file = None;
            }
            if pick {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.load_file_for_send(path);
                }
            }
            return;
        }

        // Empty drop zone: dashed outline, one clear action.
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), EDITOR_MIN_HEIGHT),
            egui::Sense::hover(),
        );
        dashed_rect(ui.painter(), rect, t.border_card_light);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(8.0)).layout(Layout::top_down(Align::Center)));
        child.add_space(((rect.height() - 92.0) / 2.0).max(0.0));
        theme::icon(&mut child, Icon::Upload, 20.0, t.text_muted);
        child.add_space(2.0);
        child.label(RichText::new("把文件拖到这里").size(13.5).color(t.text_secondary));
        child.add_space(2.0);
        if theme::secondary_button(&mut child, "选择文件", &t).clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                self.load_file_for_send(path);
            }
        }
    }

    fn render_password_input(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card(ui, &t, |ui| {
            ui.columns(2, |cols| {
                for (col, label, value, hint) in [
                    (0, "解密口令", &mut self.send_password, "设置口令"),
                    (1, "确认口令", &mut self.send_confirm, "再次输入"),
                ] {
                    let ui = &mut cols[col];
                    ui.spacing_mut().item_spacing.y = 4.0;
                    theme::field_label(ui, label, &t);
                    ui.add(
                        egui::TextEdit::singleline(value)
                            .password(!self.send_show_password)
                            .hint_text(hint)
                            .margin(egui::vec2(8.0, 6.0))
                            .desired_width(f32::INFINITY),
                    );
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.send_show_password, "显示口令");
                if !self.send_confirm.is_empty() && self.send_password != self.send_confirm {
                    theme::status_label(ui, "两次口令不一致", t.accent_rose);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme::ghost_button_colored(ui, "随机生成并复制", t.text_accent, &t)
                        .on_hover_text("生成好记的随机口令，填入两栏并复制")
                        .clicked()
                    {
                        let pwd = generate_memorable_password();
                        self.send_password = pwd.clone();
                        self.send_confirm = pwd.clone();
                        self.copy_text(ui.ctx(), pwd, "口令已生成并复制，请通过另一渠道告知对方");
                    }
                });
            });
        });
    }

    fn render_recipient_selector(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card_frame(&t).inner_margin(Margin::symmetric(14.0, 10.0)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                theme::field_label(ui, "接收人", &t);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme::ghost_button_colored(ui, "添加联系人", t.text_accent, &t).clicked() {
                        self.tab = Tab::Identity;
                        self.show_add_contact_dialog = true;
                        self.focus_new_contact = true;
                    }
                });
            });
            ui.add_space(2.0);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for i in 0..self.contacts.len() {
                    let selected = self.selected_contact_idx == Some(i);
                    if chip(ui, &self.contacts[i].name, selected, &t).clicked() {
                        self.selected_contact_idx = Some(i);
                    }
                }
                let custom = self.selected_contact_idx.is_none();
                if chip(ui, "粘贴公开身份", custom, &t).clicked() {
                    self.selected_contact_idx = None;
                }
            });
            ui.add_space(6.0);

            if let Some(contact) = self.selected_contact_idx.and_then(|i| self.contacts.get(i)) {
                theme::well_frame(&t).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        theme::field_label(ui, "身份指纹", &t);
                        ui.label(
                            RichText::new(&contact.fingerprint)
                                .monospace()
                                .color(t.text_primary),
                        );
                    });
                });
                return;
            }

            theme::caption(ui, "对方的公开身份代码", &t);
            ui.add(
                egui::TextEdit::multiline(&mut self.recipient_custom_key)
                    .hint_text("-----BEGIN PUBLIC KEY-----")
                    .font(egui::TextStyle::Monospace)
                    .margin(egui::vec2(8.0, 5.0))
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );
            if !self.recipient_custom_key.trim().is_empty() {
                ui.add_space(2.0);
                match validate_public_key(&self.recipient_custom_key) {
                    Ok(fp) => {
                        theme::status_label(ui, &format!("有效 · 指纹 {fp}"), t.text_accent);
                    }
                    Err(e) => {
                        theme::status_label(ui, &e.to_string(), t.accent_rose);
                    }
                }
            }
        });
    }

    fn render_generated_output(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card(ui, &t, |ui| {
            ui.horizontal(|ui| {
                theme::icon(ui, Icon::Check, 28.0, t.accent_emerald);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(RichText::new("密信已生成").size(16.0).strong());
                    if let Some(summary) = &self.generated_summary {
                        ui.add(
                            egui::Label::new(RichText::new(summary).size(12.0).color(t.text_muted))
                                .truncate(),
                        );
                    }
                });
            });
            ui.add_space(14.0);
            ui.label(
                RichText::new(match self.send_mode {
                    SendMode::Password => "复制或保存密信，通过聊天、邮件等任意方式发给对方。",
                    SendMode::Contact => "复制或保存密信，发给对方即可。只有对方的本机身份能解开。",
                })
                .color(t.text_secondary),
            );
            if self.send_mode == SendMode::Password {
                ui.add_space(8.0);
                theme::well_frame(&t).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::Lock, 14.0, t.accent_amber);
                        ui.add(
                            egui::Label::new(
                                RichText::new("口令请通过另一渠道告知对方，不要和密信放在一起发送。")
                                    .size(12.5)
                                    .color(t.text_secondary),
                            )
                            .wrap(),
                        );
                    });
                });
            }
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if theme::primary_button(ui, "复制密信文本", &t).clicked() {
                    self.copy_text(
                        ui.ctx(),
                        self.generated_cipher.clone(),
                        "密信已复制，可直接粘贴发给对方",
                    );
                }
                if theme::secondary_button(ui, "保存密信文件", &t)
                    .on_hover_text("保存为 .mixin 文件，对方可直接拖入密信打开")
                    .clicked()
                {
                    self.save_generated_cipher();
                }
            });
            ui.add_space(14.0);
            theme::hairline(ui, &t);
            ui.add_space(6.0);
            egui::CollapsingHeader::new(
                RichText::new("查看加密文本").size(12.5).color(t.text_secondary),
            )
            .id_salt("cipher_text")
            .show(ui, |ui| {
                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.generated_cipher.as_str())
                            .font(egui::TextStyle::Monospace)
                            .margin(egui::vec2(8.0, 6.0))
                            .desired_width(f32::INFINITY),
                    );
                });
            });
        });
    }

    fn save_generated_cipher(&mut self) {
        let default_name = match &self.send_file {
            Some(file) if self.payload_type == PayloadType::File => format!("{}.mixin", file.name),
            _ => "密信.mixin".to_string(),
        };
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(default_name)
            .add_filter("密信文件", &["mixin"])
            .save_file()
        else {
            return;
        };
        match fs::write(&path, &self.generated_cipher) {
            Ok(_) => self.notify(
                format!(
                    "已保存 {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ),
                ToastKind::Success,
            ),
            Err(e) => self.notify(format!("保存失败：{e}"), ToastKind::Error),
        }
    }

    // ========================================================
    // Receive
    // ========================================================
    fn render_receive_tab(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        egui::ScrollArea::vertical()
            .id_salt("receive_scroll")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let trimmed = self.receive_input.trim();
                let detected = if trimmed.starts_with(IDENTITY_PREFIX) {
                    Some(("专属密信", t.text_accent))
                } else if trimmed.starts_with(PREFIX) {
                    Some(("口令密信", t.text_accent))
                } else if !trimmed.is_empty() {
                    Some(("无法识别", t.accent_amber))
                } else {
                    None
                };

                theme::section_header(ui, "密信", &t, |ui| {
                    if let Some((label, color)) = detected {
                        theme::status_label(ui, label, color);
                    }
                });
                theme::paper(ui, &t, |ui| {
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.receive_input)
                            .hint_text("粘贴收到的密信，或把 .mixin 文件拖进窗口")
                            .font(egui::TextStyle::Monospace)
                            .frame(false)
                            .margin(egui::vec2(0.0, 2.0))
                            .desired_rows(4)
                            .min_size(egui::vec2(0.0, EDITOR_MIN_HEIGHT - 34.0))
                            .desired_width(f32::INFINITY),
                    );
                    if response.changed() {
                        self.decrypted = None;
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if theme::ghost_button(ui, "粘贴密信", &t).clicked() {
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                if let Ok(text) = cb.get_text() {
                                    self.receive_input = text.trim().to_string();
                                    self.decrypted = None;
                                }
                            }
                        }
                        if theme::ghost_button(ui, "打开文件", &t).clicked() {
                            self.open_cipher_file();
                        }
                        if !self.receive_input.is_empty() {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if theme::ghost_button(ui, "清空", &t).clicked() {
                                    self.receive_input.clear();
                                    self.receive_password.clear();
                                    self.decrypted = None;
                                }
                            });
                        }
                    });
                });

                let trimmed = self.receive_input.trim();
                if trimmed.starts_with(IDENTITY_PREFIX) {
                    ui.add_space(16.0);
                    self.render_identity_decrypt_card(ui);
                } else if trimmed.starts_with(PREFIX) {
                    ui.add_space(16.0);
                    self.render_password_decrypt_card(ui);
                } else if !trimmed.is_empty() {
                    ui.add_space(16.0);
                    theme::card(ui, &t, |ui| {
                        ui.horizontal(|ui| {
                            theme::icon(ui, Icon::Warning, 16.0, t.accent_amber);
                            ui.label(RichText::new("这段内容不是密信").strong());
                        });
                        ui.add_space(2.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(
                                    "密信以 MX1.（口令）或 MXID1.（专属）开头，请确认复制完整。",
                                )
                                .size(12.5)
                                .color(t.text_secondary),
                            )
                            .wrap(),
                        );
                    });
                }

                if self.decrypted.is_some() {
                    ui.add_space(16.0);
                    self.render_decrypted_result(ui);
                }
                ui.add_space(8.0);
            });
    }

    fn open_cipher_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("密信文件", &["mixin", "txt"])
            .pick_file()
        else {
            return;
        };
        match fs::read_to_string(&path) {
            Ok(content) => {
                self.receive_input = content.trim().to_string();
                self.decrypted = None;
            }
            Err(e) => self.notify(format!("无法读取文件：{e}"), ToastKind::Error),
        }
    }

    fn render_identity_decrypt_card(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card(ui, &t, |ui| {
            ui.horizontal(|ui| {
                theme::icon(ui, Icon::Identity, 16.0, t.text_accent);
                ui.label(RichText::new("这封密信只有本机身份能打开").strong());
            });
            ui.add_space(2.0);
            theme::caption(ui, "无需口令，确认后直接解密。", &t);
            ui.add_space(10.0);
            if theme::primary_button(ui, "使用本机身份解密", &t).clicked() {
                self.do_receive();
            }
        });
    }

    fn render_password_decrypt_card(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card(ui, &t, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            theme::field_label(ui, "解密口令", &t);
            ui.horizontal(|ui| {
                let can_decrypt = !self.receive_password.is_empty();
                let field_width = (ui.available_width() - 200.0).max(120.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.receive_password)
                        .password(!self.receive_show_password)
                        .hint_text("输入对方告知的口令")
                        .margin(egui::vec2(8.0, 6.0))
                        .desired_width(field_width),
                );
                let submitted = response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && can_decrypt;
                ui.checkbox(&mut self.receive_show_password, "显示");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary_action(ui, "解密密信", can_decrypt, &t).clicked() || submitted {
                        self.do_receive();
                    }
                });
            });
        });
    }

    fn render_decrypted_result(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        let Some(content) = self.decrypted.clone() else {
            return;
        };
        theme::section_header(ui, "解密结果", &t, |_| {});
        theme::card(ui, &t, |ui| match content {
            DecryptedContent::Text(plain_text) => {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::Check, 18.0, t.accent_emerald);
                    ui.label(RichText::new("文字").strong());
                    theme::caption(ui, format!("{} 字", plain_text.chars().count()), &t);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::primary_button(ui, "复制文字", &t).clicked() {
                            self.copy_text(ui.ctx(), plain_text.clone(), "已复制解密文字");
                        }
                    });
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .id_salt("decrypted_text")
                    .max_height(220.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut plain_text.as_str())
                                .desired_rows(5)
                                .margin(egui::vec2(10.0, 8.0))
                                .desired_width(f32::INFINITY),
                        );
                    });
            }
            DecryptedContent::File {
                meta,
                bytes,
                saved_path,
            } => {
                let full_filename = display_name(&meta.name, &meta.extension);
                let mut save = false;
                ui.horizontal(|ui| {
                    file_tile(ui, &meta.extension, &t);
                    ui.add_space(4.0);
                    let buttons_width = 120.0;
                    ui.allocate_ui_with_layout(
                        egui::vec2((ui.available_width() - buttons_width).max(80.0), 44.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.add(
                                egui::Label::new(RichText::new(&full_filename).strong())
                                    .truncate(),
                            )
                            .on_hover_text(&full_filename);
                            match &saved_path {
                                Some(path) => {
                                    let text = format!("已保存到 {}", path.display());
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&text).size(12.0).color(t.text_accent),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(text);
                                }
                                None => {
                                    theme::caption(
                                        ui,
                                        format!("{} · 尚未保存", format_file_size(bytes.len())),
                                        &t,
                                    );
                                }
                            }
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        save = if saved_path.is_some() {
                            theme::secondary_button(ui, "另存为", &t).clicked()
                        } else {
                            theme::primary_button(ui, "保存文件", &t).clicked()
                        };
                    });
                });
                if save {
                    if let Some(dest) = rfd::FileDialog::new()
                        .set_file_name(&full_filename)
                        .save_file()
                    {
                        match fs::write(&dest, &bytes) {
                            Ok(_) => {
                                self.notify(
                                    format!(
                                        "已保存 {}",
                                        dest.file_name().unwrap_or_default().to_string_lossy()
                                    ),
                                    ToastKind::Success,
                                );
                                self.decrypted = Some(DecryptedContent::File {
                                    meta,
                                    bytes,
                                    saved_path: Some(dest),
                                });
                            }
                            Err(e) => self.notify(format!("保存失败：{e}"), ToastKind::Error),
                        }
                    }
                }
            }
        });
    }

    // ========================================================
    // Identity & contacts
    // ========================================================
    fn render_identity_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("identity_scroll")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                self.render_my_identity_card(ui);
                ui.add_space(20.0);
                self.render_address_book(ui);
                ui.add_space(20.0);
                self.render_identity_maintenance(ui);
                ui.add_space(8.0);
            });
    }

    fn render_my_identity_card(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::section_header(ui, "我的公开身份", &t, |_| {});
        theme::card(ui, &t, |ui| {
            let qr_size = 136.0;
            ui.horizontal_top(|ui| {
                let text_width = (ui.available_width() - qr_size - 24.0).max(200.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(text_width, qr_size),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.set_width(text_width);
                        ui.add(
                            egui::Label::new(
                                RichText::new("把公开身份发给对方，对方就能给你发专属密信，不用约定口令。")
                                    .size(13.0)
                                    .color(t.text_secondary),
                            )
                            .wrap(),
                        );
                        ui.add_space(12.0);
                        theme::field_label(ui, "身份指纹", &t);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&self.my_fingerprint)
                                    .size(15.0)
                                    .monospace()
                                    .strong()
                                    .color(t.text_primary),
                            )
                            .on_hover_text("当面或电话核对指纹，可确认身份没有被冒充");
                            if theme::ghost_button(ui, "复制", &t).clicked() {
                                self.copy_text(ui.ctx(), self.my_fingerprint.clone(), "已复制指纹");
                            }
                        });
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            if theme::primary_button(ui, "复制公开身份", &t).clicked() {
                                self.copy_text(
                                    ui.ctx(),
                                    self.identity.1.clone(),
                                    "已复制公开身份，发给对方即可",
                                );
                            }
                            if theme::secondary_button(ui, "保存二维码", &t).clicked() {
                                self.save_qr();
                            }
                        });
                    },
                );
                ui.add_space(12.0);
                egui::Frame::none()
                    .stroke(Stroke::new(1.0, t.border_card))
                    .rounding(10.0)
                    .show(ui, |ui| theme::render_qr(ui, &self.identity.1, qr_size));
            });
            ui.add_space(12.0);
            theme::hairline(ui, &t);
            ui.add_space(6.0);
            egui::CollapsingHeader::new(
                RichText::new("查看公开身份代码").size(12.5).color(t.text_secondary),
            )
            .id_salt("public_identity_code")
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.identity.1.as_str())
                        .font(egui::TextStyle::Monospace)
                        .margin(egui::vec2(8.0, 6.0))
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                );
            });
        });
    }

    fn save_qr(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("mixin_qr.png")
            .add_filter("PNG 图片", &["png"])
            .save_file()
        {
            match theme::export_qr_png(&self.identity.1, &path) {
                Ok(_) => self.notify("二维码已保存", ToastKind::Success),
                Err(e) => self.notify(e, ToastKind::Error),
            }
        }
    }

    fn render_address_book(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        let count = self.contacts.len();
        let mut toggle_form = false;
        theme::section_header(
            ui,
            &if count > 0 {
                format!("常用联系人（共 {count} 人）")
            } else {
                "常用联系人".to_string()
            },
            &t,
            |ui| {
                if !self.show_add_contact_dialog {
                    toggle_form = theme::secondary_button(ui, "添加联系人", &t).clicked();
                }
            },
        );
        if toggle_form {
            self.show_add_contact_dialog = true;
            self.focus_new_contact = true;
        }

        if self.show_add_contact_dialog {
            self.render_add_contact_form(ui);
            ui.add_space(10.0);
        }

        if self.contacts.is_empty() {
            if !self.show_add_contact_dialog {
                theme::card(ui, &t, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(6.0);
                        theme::icon(ui, Icon::Identity, 20.0, t.text_muted);
                        ui.label(RichText::new("还没有联系人").color(t.text_secondary));
                        theme::caption(ui, "保存对方的公开身份后，发送时一键选择。", &t);
                        ui.add_space(6.0);
                    });
                });
            }
            return;
        }

        let mut send_to = None;
        let mut copy_key = None;
        let mut delete_idx = None;
        theme::card_frame(&t)
            .inner_margin(Margin::symmetric(6.0, 6.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 0.0;
                for (i, contact) in self.contacts.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(2.0);
                        theme::hairline(ui, &t);
                        ui.add_space(2.0);
                    }
                    let row = egui::Frame::none()
                        .inner_margin(Margin::symmetric(10.0, 8.0))
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(4.0, 2.0);
                                let confirming = self.confirm_delete_idx == Some(i);
                                let actions_width = if confirming { 150.0 } else { 220.0 };
                                ui.allocate_ui_with_layout(
                                    egui::vec2(
                                        (ui.available_width() - actions_width).max(80.0),
                                        38.0,
                                    ),
                                    Layout::top_down(Align::Min),
                                    |ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&contact.name).strong(),
                                            )
                                            .truncate(),
                                        );
                                        ui.label(
                                            RichText::new(&contact.fingerprint)
                                                .monospace()
                                                .size(11.5)
                                                .color(t.text_muted),
                                        );
                                    },
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if confirming {
                                        if theme::ghost_button(ui, "取消", &t).clicked() {
                                            self.confirm_delete_idx = None;
                                        }
                                        if theme::danger_button(ui, "确认删除", &t).clicked() {
                                            delete_idx = Some(i);
                                        }
                                    } else {
                                        if theme::ghost_button_colored(ui, "删除", t.accent_rose, &t)
                                            .clicked()
                                        {
                                            self.confirm_delete_idx = Some(i);
                                        }
                                        if theme::ghost_button(ui, "复制公开身份", &t).clicked() {
                                            copy_key = Some(contact.public_key.clone());
                                        }
                                        if theme::ghost_button_colored(
                                            ui,
                                            "发送密信",
                                            t.text_accent,
                                            &t,
                                        )
                                        .clicked()
                                        {
                                            send_to = Some(i);
                                        }
                                    }
                                });
                            });
                        });
                    let _ = row;
                }
            });

        if let Some(key) = copy_key {
            self.copy_text(ui.ctx(), key, "已复制联系人的公开身份");
        }
        if let Some(i) = send_to {
            self.tab = Tab::Send;
            self.generated_cipher.clear();
            self.generated_summary = None;
            self.send_mode = SendMode::Contact;
            self.selected_contact_idx = Some(i);
        }
        if let Some(i) = delete_idx {
            let name = self.contacts[i].name.clone();
            remove_contact(&mut self.contacts, i);
            self.confirm_delete_idx = None;
            self.selected_contact_idx = self.selected_contact_idx.and_then(|selected| {
                if selected == i {
                    None
                } else {
                    Some(if selected > i { selected - 1 } else { selected })
                }
            });
            self.notify(format!("已删除联系人 {name}"), ToastKind::Info);
        }
    }

    fn render_add_contact_form(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        theme::card(ui, &t, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            theme::field_label(ui, "名称", &t);
            let name = ui.add(
                egui::TextEdit::singleline(&mut self.new_contact_name)
                    .hint_text("例如：小王、家里的电脑")
                    .margin(egui::vec2(8.0, 6.0))
                    .desired_width(f32::INFINITY),
            );
            if std::mem::take(&mut self.focus_new_contact) {
                name.request_focus();
                name.scroll_to_me(Some(Align::Center));
            }
            ui.add_space(6.0);
            theme::field_label(ui, "公开身份代码", &t);
            ui.add(
                egui::TextEdit::multiline(&mut self.new_contact_key)
                    .hint_text("-----BEGIN PUBLIC KEY-----")
                    .font(egui::TextStyle::Monospace)
                    .margin(egui::vec2(8.0, 6.0))
                    .desired_rows(3)
                    .desired_width(f32::INFINITY),
            );
            let fingerprint = validate_public_key(&self.new_contact_key);
            if !self.new_contact_key.trim().is_empty() {
                match &fingerprint {
                    Ok(fp) => theme::status_label(ui, &format!("有效 · 指纹 {fp}"), t.text_accent),
                    Err(e) => theme::status_label(ui, &e.to_string(), t.accent_rose),
                };
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let can_save = !self.new_contact_name.trim().is_empty() && fingerprint.is_ok();
                if primary_action(ui, "保存联系人", can_save, &t).clicked() {
                    match add_contact(
                        &mut self.contacts,
                        &self.new_contact_name,
                        &self.new_contact_key,
                    ) {
                        Ok(c) => {
                            self.notify(format!("已添加联系人 {}", c.name), ToastKind::Success);
                            self.new_contact_name.clear();
                            self.new_contact_key.clear();
                            self.show_add_contact_dialog = false;
                        }
                        Err(e) => self.notify(format!("添加失败：{e}"), ToastKind::Error),
                    }
                }
                if theme::ghost_button(ui, "取消", &t).clicked() {
                    self.show_add_contact_dialog = false;
                }
            });
        });
    }

    fn render_identity_maintenance(&mut self, ui: &mut egui::Ui) {
        let t = self.theme();
        egui::CollapsingHeader::new(
            RichText::new("备份与恢复").size(13.0).strong().color(t.text_secondary),
        )
        .id_salt("identity_maintenance")
        .show_unindented(ui, |ui| {
            ui.add_space(4.0);
            theme::card(ui, &t, |ui| {
                let mut backup = false;
                let mut import = false;
                setting_row(ui, &t, "备份私钥", "换电脑或重装系统前保存，文件请妥善保管。", |ui| {
                    backup = theme::secondary_button(ui, "备份", &t).clicked();
                });
                theme::hairline(ui, &t);
                setting_row(ui, &t, "导入私钥", "从备份恢复身份，会替换当前身份。", |ui| {
                    import = theme::secondary_button(ui, "导入", &t).clicked();
                });
                theme::hairline(ui, &t);
                setting_row(
                    ui,
                    &t,
                    "重置本机身份",
                    "生成新的身份，发给旧身份的密信将无法再解开。",
                    |ui| {
                        if !self.show_regenerate_confirm
                            && theme::ghost_button_colored(ui, "重置", t.accent_rose, &t).clicked()
                        {
                            self.show_regenerate_confirm = true;
                        }
                    },
                );
                if self.show_regenerate_confirm {
                    ui.add_space(4.0);
                    let rose = t.accent_rose;
                    egui::Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(rose.r(), rose.g(), rose.b(), 18))
                        .rounding(8.0)
                        .inner_margin(Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                theme::icon(ui, Icon::Warning, 16.0, rose);
                                ui.label(RichText::new("确定重置本机身份？").strong().color(rose));
                            });
                            ui.add(
                                egui::Label::new(
                                    RichText::new("此操作无法撤销。建议先备份私钥，并把新的公开身份重新发给联系人。")
                                        .size(12.5)
                                        .color(t.text_secondary),
                                )
                                .wrap(),
                            );
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if theme::danger_button(ui, "确认重置", &t).clicked() {
                                    self.regenerate_identity();
                                }
                                if theme::ghost_button(ui, "取消", &t).clicked() {
                                    self.show_regenerate_confirm = false;
                                }
                            });
                        });
                }
                if backup {
                    self.backup_identity();
                }
                if import {
                    self.import_identity();
                }
            });
        });
    }

    fn backup_identity(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("mixin_identity.pem")
            .save_file()
        {
            match fs::write(&path, &self.identity.0) {
                Ok(_) => self.notify("私钥已备份，请妥善保管该文件", ToastKind::Success),
                Err(e) => self.notify(format!("备份失败：{e}"), ToastKind::Error),
            }
        }
    }

    fn import_identity(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PEM", &["pem", "txt"])
            .pick_file()
        else {
            return;
        };
        let priv_pem = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) => return self.notify(format!("无法读取文件：{e}"), ToastKind::Error),
        };
        match public_from_private(&priv_pem) {
            Ok(pub_pem) => self.adopt_identity((priv_pem, pub_pem), "已导入本机身份"),
            Err(e) => self.notify(format!("无效的私钥文件：{e}"), ToastKind::Error),
        }
    }

    fn regenerate_identity(&mut self) {
        match generate_identity() {
            Ok(identity) => {
                self.show_regenerate_confirm = false;
                self.adopt_identity(identity, "已生成新的本机身份");
            }
            Err(e) => self.notify(format!("生成失败：{e}"), ToastKind::Error),
        }
    }

    fn adopt_identity(&mut self, identity: (String, String), success: &str) {
        if let Err(e) = replace_identity(&identity.0) {
            self.notify(format!("身份保存失败：{e}"), ToastKind::Error);
            return;
        }
        self.identity = identity;
        self.my_fingerprint = compute_fingerprint(&self.identity.1);
        self.notify(success, ToastKind::Success);
    }
}

// ============================================================
// Small view helpers
// ============================================================

fn display_name(name: &str, extension: &str) -> String {
    if extension.is_empty() {
        name.to_string()
    } else {
        format!("{name}.{extension}")
    }
}

/// Primary action that stays legible when unavailable: neutral fill, muted text, no click.
fn primary_action(ui: &mut egui::Ui, label: &str, enabled: bool, t: &ThemeColors) -> egui::Response {
    if enabled {
        return theme::primary_button(ui, label, t);
    }
    ui.scope(|ui| {
        let v = ui.visuals_mut();
        v.override_text_color = Some(t.text_muted);
        for state in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
        ] {
            state.weak_bg_fill = t.bg_card_elevated;
            state.bg_stroke = Stroke::NONE;
            state.rounding = egui::Rounding::same(theme::RADIUS);
            state.expansion = 0.0;
        }
        ui.add(
            egui::Button::new(RichText::new(label).strong())
                .sense(egui::Sense::hover())
                .min_size(egui::vec2(96.0, theme::PRIMARY_HEIGHT)),
        )
    })
    .inner
}

/// Selectable pill for recipients.
fn chip(ui: &mut egui::Ui, label: &str, selected: bool, t: &ThemeColors) -> egui::Response {
    let response = ui
        .scope(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(12.0, 4.0);
            let v = ui.visuals_mut();
            v.override_text_color = Some(if selected { t.text_accent } else { t.text_primary });
            for (i, state) in [
                &mut v.widgets.inactive,
                &mut v.widgets.hovered,
                &mut v.widgets.active,
            ]
            .into_iter()
            .enumerate()
            {
                state.weak_bg_fill = if selected {
                    t.bg_selection
                } else if i == 0 {
                    t.bg_card
                } else {
                    t.bg_card_hover
                };
                state.bg_stroke = Stroke::new(1.0, if selected { t.border_accent } else { t.border_card_light });
                state.rounding = egui::Rounding::same(14.0);
                state.expansion = 0.0;
            }
            ui.add(egui::Button::new(RichText::new(label).size(13.0)).min_size(egui::vec2(0.0, 28.0)))
        })
        .inner;
    theme::focus_outline(ui, &response, t);
    response
}

/// File glyph with the extension underneath, used for selected and decrypted files.
fn file_tile(ui: &mut egui::Ui, extension: &str, t: &ThemeColors) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(40.0, 44.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, t.bg_selection);
    theme::paint_icon(
        painter,
        egui::Rect::from_center_size(rect.center() - egui::vec2(0.0, 6.0), egui::vec2(16.0, 16.0)),
        Icon::File,
        t.text_accent,
    );
    let ext: String = extension.chars().take(4).collect::<String>().to_uppercase();
    if !ext.is_empty() {
        painter.text(
            rect.center() + egui::vec2(0.0, 11.0),
            Align2::CENTER_CENTER,
            ext,
            egui::FontId::proportional(9.5),
            t.text_accent,
        );
    }
}

fn dashed_rect(painter: &egui::Painter, rect: egui::Rect, color: Color32) {
    let r = rect.shrink(0.5);
    let stroke = Stroke::new(1.0, color);
    let corners = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    for pair in corners.windows(2) {
        painter.extend(egui::Shape::dashed_line(&[pair[0], pair[1]], stroke, 5.0, 4.0));
    }
}

/// Settings-style row: title and description on the left, a control on the right.
fn setting_row(
    ui: &mut egui::Ui,
    t: &ThemeColors,
    title: &str,
    description: &str,
    control: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.set_min_height(48.0);
        let control_width = 90.0;
        ui.allocate_ui_with_layout(
            egui::vec2((ui.available_width() - control_width).max(120.0), 40.0),
            Layout::top_down(Align::Min),
            |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(title).size(13.5));
                ui.add(
                    egui::Label::new(RichText::new(description).size(12.0).color(t.text_muted))
                        .wrap(),
                );
            },
        );
        ui.with_layout(Layout::right_to_left(Align::Center), control);
    });
}
