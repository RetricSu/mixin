#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! 密信：在两台电脑之间传一句秘密。

mod crypto;

use crypto::{decrypt, encrypt};
use eframe::egui;

#[derive(Default, PartialEq)]
enum Screen {
    #[default]
    Home,
    Encrypt,
    Decrypt,
}

#[derive(Default)]
struct SecretMessageApp {
    screen: Screen,
    password: String,
    password_confirmation: String,
    message: String,
    token: String,
    error: String,
    notice: String,
}

impl SecretMessageApp {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn copy(&mut self, value: &str) {
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(value)) {
            Ok(()) => {
                self.error.clear();
                self.notice = "已复制到剪贴板，可以粘贴到聊天窗口。".into();
            }
            Err(_) => self.error = "复制失败，请手动选中内容复制。".into(),
        }
    }

    fn heading(ui: &mut egui::Ui, title: &str, description: &str) {
        ui.heading(title);
        ui.add_space(4.0);
        ui.label(egui::RichText::new(description).color(egui::Color32::from_gray(110)));
        ui.add_space(22.0);
    }

    fn password_field(ui: &mut egui::Ui, label: &str, value: &mut String) {
        ui.label(label);
        ui.add_space(5.0);
        ui.add_sized(
            [ui.available_width(), 38.0],
            egui::TextEdit::singleline(value)
                .password(true)
                .hint_text("输入双方约定的密码"),
        );
        ui.add_space(14.0);
    }

    fn feedback(&self, ui: &mut egui::Ui) {
        if !self.error.is_empty() {
            ui.add_space(12.0);
            ui.colored_label(egui::Color32::from_rgb(180, 55, 45), &self.error);
        } else if !self.notice.is_empty() {
            ui.add_space(12.0);
            ui.colored_label(egui::Color32::from_rgb(45, 125, 85), &self.notice);
        }
    }

    fn home(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(32.0);
            ui.label(egui::RichText::new("密信").size(38.0).strong());
            ui.add_space(8.0);
            ui.label("两台电脑，同一个密码，传一句秘密");
            ui.add_space(38.0);
            if ui
                .add_sized([270.0, 48.0], egui::Button::new("加密一段话"))
                .clicked()
            {
                self.screen = Screen::Encrypt;
                self.error.clear();
                self.notice.clear();
            }
            ui.add_space(12.0);
            if ui
                .add_sized([270.0, 48.0], egui::Button::new("解密收到的密文"))
                .clicked()
            {
                self.screen = Screen::Decrypt;
                self.error.clear();
                self.notice.clear();
            }
            ui.add_space(42.0);
            ui.label(
                egui::RichText::new("密码请当面说或打电话说，不要和密文发在同一个聊天里。")
                    .small()
                    .color(egui::Color32::from_gray(115)),
            );
        });
    }

    fn encrypt_screen(&mut self, ui: &mut egui::Ui) {
        Self::heading(ui, "加密", "把一句话变成可以安全发送的密文。");
        Self::password_field(ui, "设一个密码", &mut self.password);
        Self::password_field(ui, "确认密码", &mut self.password_confirmation);
        ui.label("要保密的话");
        ui.add_space(5.0);
        ui.add_sized(
            [ui.available_width(), 105.0],
            egui::TextEdit::multiline(&mut self.message).hint_text("写下要传给对方的话"),
        );
        ui.add_space(18.0);
        if ui
            .add_sized([ui.available_width(), 42.0], egui::Button::new("生成密文"))
            .clicked()
        {
            self.error.clear();
            self.notice.clear();
            if self.password.is_empty() {
                self.error = "密码不能为空。".into();
            } else if self.password != self.password_confirmation {
                self.error = "两次输入的密码不一样。".into();
            } else if self.message.trim().is_empty() {
                self.error = "要保密的话不能为空。".into();
            } else {
                match encrypt(&self.password, &self.message) {
                    Ok(token) => {
                        self.token = token;
                        let token = self.token.clone();
                        self.copy(&token);
                    }
                    Err(error) => self.error = error,
                }
            }
        }
        if !self.token.is_empty() {
            ui.add_space(18.0);
            ui.label("密文");
            ui.add_space(5.0);
            ui.add(egui::TextEdit::multiline(&mut self.token).desired_rows(3));
            if ui.button("再次复制").clicked() {
                let token = self.token.clone();
                self.copy(&token);
            }
        }
        self.feedback(ui);
    }

    fn decrypt_screen(&mut self, ui: &mut egui::Ui) {
        Self::heading(ui, "解密", "粘贴对方发来的密文，查看原文。");
        Self::password_field(ui, "密码", &mut self.password);
        ui.label("收到的密文");
        ui.add_space(5.0);
        ui.add_sized(
            [ui.available_width(), 125.0],
            egui::TextEdit::multiline(&mut self.token).hint_text("粘贴 MX1. 开头的整段内容"),
        );
        ui.add_space(18.0);
        if ui
            .add_sized([ui.available_width(), 42.0], egui::Button::new("解密"))
            .clicked()
        {
            self.error.clear();
            self.notice.clear();
            if self.password.is_empty() {
                self.error = "密码不能为空。".into();
            } else if self.token.trim().is_empty() {
                self.error = "请先粘贴收到的密文。".into();
            } else {
                match decrypt(&self.password, &self.token) {
                    Ok(message) => {
                        self.message = message;
                        self.notice = "解密成功。".into();
                    }
                    Err(error) => self.error = error,
                }
            }
        }
        if !self.message.is_empty() {
            ui.add_space(18.0);
            ui.label("原文");
            ui.add_space(5.0);
            ui.add(egui::TextEdit::multiline(&mut self.message).desired_rows(4));
            if ui.button("复制原文").clicked() {
                let message = self.message.clone();
                self.copy(&message);
            }
        }
        self.feedback(ui);
    }
}

impl eframe::App for SecretMessageApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.set_max_width(560.0);
        ui.add_space(24.0);
        if self.screen != Screen::Home && ui.button("‹ 返回首页").clicked() {
            self.reset();
        }
        match self.screen {
            Screen::Home => self.home(ui),
            Screen::Encrypt => self.encrypt_screen(ui),
            Screen::Decrypt => self.decrypt_screen(ui),
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([620.0, 650.0])
            .with_min_inner_size([480.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "密信",
        options,
        Box::new(|_creation_context| Ok(Box::new(SecretMessageApp::default()))),
    )
}
