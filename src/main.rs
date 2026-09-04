//! 密信桌面版：把“算法”藏在简单的文字与文件传递流程后面。
mod crypto;

use base64::Engine as _;
use crypto::{
    decrypt_payload, decrypt_with, encrypt_for, generate_identity, public_from_private, Envelope,
    FileMeta, IDENTITY_PREFIX,
};
use eframe::egui;
use qrcode::{types::Color, QrCode};
use std::{fs, path::PathBuf};

#[derive(PartialEq)]
enum Tab {
    Send,
    Receive,
    Identity,
}
struct App {
    tab: Tab,
    mode: usize,
    text: String,
    result: String,
    secret: String,
    confirm: String,
    recipient: String,
    file: Option<PathBuf>,
    identity: (String, String),
    notice: String,
}

impl App {
    fn new() -> Self {
        let identity =
            load_identity().unwrap_or_else(|| generate_identity().expect("无法生成本机身份"));
        save_identity(&identity.0);
        Self {
            tab: Tab::Send,
            mode: 0,
            text: String::new(),
            result: String::new(),
            secret: String::new(),
            confirm: String::new(),
            recipient: String::new(),
            file: None,
            identity,
            notice: "本机身份已准备好".into(),
        }
    }
    fn send(&mut self, ui: &mut egui::Ui) {
        ui.heading("发一份安全内容");
        ui.label("选择传文字，或把文本文件封装后发送。接收方无需安装额外工具。");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, 0, "约定一个口令");
            ui.selectable_value(&mut self.mode, 1, "发给一位联系人");
        });
        if self.mode == 1 {
            ui.label("粘贴对方的身份信息，或扫码后的文字");
            ui.add(egui::TextEdit::multiline(&mut self.recipient).desired_rows(3));
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("选择文本文件").clicked() {
                self.file = rfd::FileDialog::new()
                    .add_filter("文本", &["txt", "md", "csv", "json", "log"])
                    .pick_file();
            }
            if let Some(path) = &self.file {
                ui.label(path.file_name().unwrap_or_default().to_string_lossy());
            }
        });
        if self.file.is_none() {
            ui.add(
                egui::TextEdit::multiline(&mut self.text)
                    .hint_text("写下要传递的内容…")
                    .desired_rows(8),
            );
        }
        if self.mode == 0 {
            ui.horizontal(|ui| {
                ui.label("口令");
                ui.add(egui::TextEdit::singleline(&mut self.secret).password(true));
                ui.label("再次输入");
                ui.add(egui::TextEdit::singleline(&mut self.confirm).password(true));
            });
        }
        if ui.button("生成可发送内容").clicked() {
            self.do_send();
        }
        if !self.result.is_empty() {
            ui.label("生成成功，下面内容可复制发送：");
            ui.add(egui::TextEdit::multiline(&mut self.result).desired_rows(6));
            if ui.button("复制").clicked() {
                ui.ctx().copy_text(self.result.clone());
                self.notice = "已复制".into();
            }
        }
    }
    fn do_send(&mut self) {
        let (bytes, meta) = match &self.file {
            Some(path) => match fs::read(path) {
                Ok(bytes) => (
                    bytes,
                    Some(FileMeta {
                        name: path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into(),
                        extension: path
                            .extension()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into(),
                        content_type: "text/plain".into(),
                    }),
                ),
                Err(e) => {
                    self.notice = format!("读取文件失败：{e}");
                    return;
                }
            },
            None => (self.text.as_bytes().to_vec(), None),
        };
        if bytes.is_empty() {
            self.notice = "内容不能为空".into();
            return;
        }
        let packet = serde_json::to_vec(&Envelope {
            kind: if meta.is_some() {
                "text-file".into()
            } else {
                "text".into()
            },
            meta,
            data: base64::engine::general_purpose::STANDARD.encode(bytes),
        })
        .unwrap();
        let out = if self.mode == 0 {
            if self.secret.is_empty() || self.secret != self.confirm {
                self.notice = "请填写相同的口令".into();
                return;
            }
            crypto::encrypt_payload(&self.secret, &packet)
        } else {
            encrypt_for(&self.recipient, &packet)
        };
        match out {
            Ok(v) => {
                self.result = v;
                self.notice = "内容已加密".into();
            }
            Err(e) => self.notice = e,
        }
    }
    fn receive(&mut self, ui: &mut egui::Ui) {
        ui.heading("打开收到的内容");
        ui.label("粘贴密文，或选择密信文件。密信只在本机解开。");
        ui.add(
            egui::TextEdit::multiline(&mut self.result)
                .hint_text("把收到的内容粘贴到这里…")
                .desired_rows(8),
        );
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, 0, "口令");
            ui.selectable_value(&mut self.mode, 1, "本机身份");
        });
        if self.mode == 0 {
            ui.add(egui::TextEdit::singleline(&mut self.secret).password(true));
        }
        if ui.button("解开内容").clicked() {
            self.do_receive();
        }
        if !self.text.is_empty() {
            ui.separator();
            ui.label("解开后的文字");
            ui.add(egui::TextEdit::multiline(&mut self.text).desired_rows(8));
            if ui.button("复制原文").clicked() {
                ui.ctx().copy_text(self.text.clone());
            }
        }
    }
    fn do_receive(&mut self) {
        let plain = if self.result.trim_start().starts_with(IDENTITY_PREFIX) {
            decrypt_with(&self.identity.0, &self.result)
        } else {
            decrypt_payload(&self.secret, &self.result)
        };
        match plain {
            Ok(bytes) if serde_json::from_slice::<Envelope>(&bytes).is_err() => {
                self.text = String::from_utf8_lossy(&bytes).into();
                self.notice = "已解开旧版文字密信".into();
            }
            Ok(bytes) => match serde_json::from_slice::<Envelope>(&bytes) {
                Ok(packet) => match base64::engine::general_purpose::STANDARD.decode(packet.data) {
                    Ok(bytes) if packet.kind == "text-file" => {
                        let meta = packet.meta.unwrap_or(FileMeta {
                            name: "解密文件".into(),
                            extension: "txt".into(),
                            content_type: "text/plain".into(),
                        });
                        let filename = if meta.extension.is_empty() {
                            meta.name
                        } else {
                            format!("{}.{}", meta.name, meta.extension)
                        };
                        if let Some(path) =
                            rfd::FileDialog::new().set_file_name(filename).save_file()
                        {
                            match fs::write(path, bytes) {
                                Ok(_) => self.notice = "文件已解开并保存".into(),
                                Err(e) => self.notice = format!("保存文件失败：{e}"),
                            }
                        }
                    }
                    Ok(bytes) => {
                        self.text = String::from_utf8_lossy(&bytes).into();
                        self.notice = "已解开".into();
                    }
                    Err(_) => self.notice = "内容数据损坏。".into(),
                },
                Err(_) => self.notice = "旧版或损坏的密信，无法读取。".into(),
            },
            Err(e) => self.notice = e,
        }
    }
    fn identity(&mut self, ui: &mut egui::Ui) {
        ui.heading("我的安全身份");
        ui.label("这是本机自动生成的一对身份信息。把下面的公开部分发给联系人，对方就能只发给你。");
        ui.label("公开身份信息（可复制或扫码）");
        ui.add(egui::TextEdit::multiline(&mut self.identity.1).desired_rows(7));
        if ui.button("复制公开身份").clicked() {
            ui.ctx().copy_text(self.identity.1.clone());
            self.notice = "已复制公开身份".into();
        }
        ui.separator();
        ui.label("二维码预览");
        qr(ui, &self.identity.1);
        ui.small("私密身份只保存在本机，位置：应用数据目录。不要发送给任何人。");
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("密信");
                ui.label("把重要内容安全地交给对方");
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.selectable_label(self.tab == Tab::Send, "发送").clicked() {
                    self.tab = Tab::Send;
                }
                if ui
                    .selectable_label(self.tab == Tab::Receive, "接收")
                    .clicked()
                {
                    self.tab = Tab::Receive;
                }
                if ui
                    .selectable_label(self.tab == Tab::Identity, "我的身份")
                    .clicked()
                {
                    self.tab = Tab::Identity;
                }
            });
            ui.separator();
            match self.tab {
                Tab::Send => self.send(ui),
                Tab::Receive => self.receive(ui),
                Tab::Identity => self.identity(ui),
            }
            ui.separator();
            ui.colored_label(egui::Color32::from_rgb(30, 110, 90), &self.notice);
        });
    }
}

fn identity_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("mixin").join("identity.pem"))
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
fn qr(ui: &mut egui::Ui, value: &str) {
    if let Ok(code) = QrCode::new(value.as_bytes()) {
        let scale = 4.0;
        let side = code.width() as f32 * scale;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        for y in 0..code.width() {
            for x in 0..code.width() {
                if code[(x, y)] == Color::Dark {
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            rect.min + egui::vec2(x as f32 * scale, y as f32 * scale),
                            egui::vec2(scale, scale),
                        ),
                        0.0,
                        egui::Color32::from_rgb(25, 30, 28),
                    );
                }
            }
        }
    }
}
fn main() -> eframe::Result {
    eframe::run_native(
        "密信",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            configure_chinese_font(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
    )
}

fn configure_chinese_font(ctx: &egui::Context) {
    let candidates = [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "C:/Windows/Fonts/msyh.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    ];
    let Some(path) = candidates
        .iter()
        .find(|path| std::path::Path::new(path).is_file())
    else {
        return;
    };
    let Ok(bytes) = fs::read(path) else { return };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("chinese".into(), egui::FontData::from_owned(bytes).into());
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "chinese".into());
    }
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_roundtrip() {
        let (private, public) = generate_identity().unwrap();
        let plain = "中文文件".as_bytes();
        let token = encrypt_for(&public, plain).unwrap();
        assert_eq!(decrypt_with(&private, &token).unwrap(), plain);
    }
}
