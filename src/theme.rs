#![allow(dead_code)]

use eframe::egui::{self, Color32, Frame, Margin, Rounding, Stroke, Ui};
use std::path::Path;

#[derive(Clone, Copy, PartialEq)]
pub struct ThemeColors {
    pub bg_canvas: Color32,
    pub bg_header: Color32,
    pub bg_card: Color32,
    pub bg_card_elevated: Color32,
    pub bg_card_hover: Color32,
    pub bg_well: Color32,

    pub border_card: Color32,
    pub border_card_light: Color32,
    pub border_accent: Color32,

    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,

    pub accent_emerald: Color32,
    pub accent_emerald_hover: Color32,
    pub accent_emerald_active: Color32,

    pub accent_sky: Color32,
    pub accent_amber: Color32,
    pub accent_rose: Color32,
}

pub const DARK_THEME: ThemeColors = ThemeColors {
    bg_canvas: Color32::from_rgb(18, 20, 26),
    bg_header: Color32::from_rgb(24, 27, 36),
    bg_card: Color32::from_rgb(26, 30, 40),
    bg_card_elevated: Color32::from_rgb(32, 38, 50),
    bg_card_hover: Color32::from_rgb(38, 45, 60),
    bg_well: Color32::from_rgb(14, 16, 22),

    border_card: Color32::from_rgb(44, 52, 68),
    border_card_light: Color32::from_rgb(64, 76, 98),
    border_accent: Color32::from_rgb(16, 185, 129),

    text_primary: Color32::from_rgb(243, 244, 246),
    text_secondary: Color32::from_rgb(156, 163, 175),
    text_muted: Color32::from_rgb(107, 114, 128),

    accent_emerald: Color32::from_rgb(16, 185, 129),
    accent_emerald_hover: Color32::from_rgb(52, 211, 153),
    accent_emerald_active: Color32::from_rgb(5, 150, 105),

    accent_sky: Color32::from_rgb(14, 165, 233),
    accent_amber: Color32::from_rgb(245, 158, 11),
    accent_rose: Color32::from_rgb(244, 63, 94),
};

pub const LIGHT_THEME: ThemeColors = ThemeColors {
    bg_canvas: Color32::from_rgb(245, 247, 250),
    bg_header: Color32::from_rgb(255, 255, 255),
    bg_card: Color32::from_rgb(255, 255, 255),
    bg_card_elevated: Color32::from_rgb(243, 245, 248),
    bg_card_hover: Color32::from_rgb(235, 238, 243),
    bg_well: Color32::from_rgb(241, 243, 247),

    border_card: Color32::from_rgb(222, 226, 233),
    border_card_light: Color32::from_rgb(205, 211, 220),
    border_accent: Color32::from_rgb(16, 185, 129),

    text_primary: Color32::from_rgb(30, 35, 45),
    text_secondary: Color32::from_rgb(85, 95, 110),
    text_muted: Color32::from_rgb(140, 148, 160),

    accent_emerald: Color32::from_rgb(16, 165, 115),
    accent_emerald_hover: Color32::from_rgb(30, 190, 135),
    accent_emerald_active: Color32::from_rgb(12, 140, 95),

    accent_sky: Color32::from_rgb(14, 145, 210),
    accent_amber: Color32::from_rgb(225, 140, 10),
    accent_rose: Color32::from_rgb(225, 45, 75),
};

// Global active theme colors shorthand
pub mod colors {
    use super::*;
    pub const BG_CANVAS: Color32 = DARK_THEME.bg_canvas;
    pub const BG_HEADER: Color32 = DARK_THEME.bg_header;
    pub const BG_CARD: Color32 = DARK_THEME.bg_card;
    pub const BG_CARD_ELEVATED: Color32 = DARK_THEME.bg_card_elevated;
    pub const BG_CARD_HOVER: Color32 = DARK_THEME.bg_card_hover;
    pub const BG_WELL: Color32 = DARK_THEME.bg_well;

    pub const BORDER_CARD: Color32 = DARK_THEME.border_card;
    pub const BORDER_CARD_LIGHT: Color32 = DARK_THEME.border_card_light;
    pub const BORDER_ACCENT: Color32 = DARK_THEME.border_accent;

    pub const TEXT_PRIMARY: Color32 = DARK_THEME.text_primary;
    pub const TEXT_SECONDARY: Color32 = DARK_THEME.text_secondary;
    pub const TEXT_MUTED: Color32 = DARK_THEME.text_muted;

    pub const ACCENT_EMERALD: Color32 = DARK_THEME.accent_emerald;
    pub const ACCENT_EMERALD_HOVER: Color32 = DARK_THEME.accent_emerald_hover;
    pub const ACCENT_EMERALD_ACTIVE: Color32 = DARK_THEME.accent_emerald_active;

    pub const ACCENT_SKY: Color32 = DARK_THEME.accent_sky;
    pub const ACCENT_AMBER: Color32 = DARK_THEME.accent_amber;
    pub const ACCENT_ROSE: Color32 = DARK_THEME.accent_rose;
}

pub fn apply_theme(ctx: &egui::Context, dark_mode: bool) {
    let t = if dark_mode { DARK_THEME } else { LIGHT_THEME };

    let mut visuals = if dark_mode {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.override_text_color = Some(t.text_primary);
    visuals.panel_fill = t.bg_canvas;
    visuals.window_fill = t.bg_card;
    visuals.extreme_bg_color = t.bg_well;
    visuals.faint_bg_color = t.bg_header;
    visuals.code_bg_color = t.bg_well;

    visuals.widgets.noninteractive.bg_fill = t.bg_card;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.border_card);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text_primary);

    visuals.widgets.inactive.bg_fill = t.bg_well;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, t.border_card);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text_secondary);
    visuals.widgets.inactive.rounding = Rounding::same(7.0);

    visuals.widgets.hovered.bg_fill = t.bg_card_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, t.border_card_light);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, t.text_primary);
    visuals.widgets.hovered.rounding = Rounding::same(7.0);

    visuals.widgets.active.bg_fill = t.accent_emerald_active;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, t.accent_emerald);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(7.0);

    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(16, 185, 129, 60);
    visuals.selection.stroke = Stroke::new(1.5, t.accent_emerald);

    visuals.window_rounding = Rounding::same(10.0);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.5);
    style.spacing.window_margin = Margin::symmetric(24.0, 16.0);

    use egui::{FontFamily, FontId, TextStyle};
    style.text_styles = [
        (TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional)),
        (
            TextStyle::Name("Subheading".into()),
            FontId::new(15.5, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
        (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
    ]
    .into();

    ctx.set_style(style);
}

pub fn configure_fonts(ctx: &egui::Context) {
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
    let Ok(bytes) = std::fs::read(path) else { return };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("chinese".into(), egui::FontData::from_owned(bytes));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "chinese".into());
    }
    ctx.set_fonts(fonts);
}

pub fn card_frame(t: &ThemeColors) -> Frame {
    Frame::none()
        .fill(t.bg_card)
        .stroke(Stroke::new(1.0, t.border_card))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(14.0))
}

pub fn highlighted_card_frame(t: &ThemeColors, accent_color: Color32) -> Frame {
    Frame::none()
        .fill(t.bg_card_elevated)
        .stroke(Stroke::new(1.5, accent_color))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(14.0))
}

pub fn well_frame(t: &ThemeColors) -> Frame {
    Frame::none()
        .fill(t.bg_well)
        .stroke(Stroke::new(1.0, t.border_card))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(10.0))
}

pub fn pill_badge(ui: &mut Ui, text: &str, color: Color32) {
    let bg_color = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 35);
    let border_color = Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 120);
    Frame::none()
        .fill(bg_color)
        .stroke(Stroke::new(1.0, border_color))
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::symmetric(8.0, 3.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(color).size(11.5).strong());
        });
}

pub fn render_qr(ui: &mut Ui, value: &str, desired_size: f32) {
    use qrcode::types::Color;
    use qrcode::QrCode;
    let Ok(code) = QrCode::new(value.as_bytes()) else {
        ui.label("无法生成二维码");
        return;
    };
    let width = code.width();
    let margin = 12.0;
    let qr_drawing_size = desired_size - margin * 2.0;
    let scale = qr_drawing_size / width as f32;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(desired_size, desired_size), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Clean white rounded card background
    painter.rect_filled(
        rect,
        Rounding::same(8.0),
        Color32::from_rgb(255, 255, 255),
    );

    // Draw QR modules
    let qr_origin = rect.min + egui::vec2(margin, margin);
    for y in 0..width {
        for x in 0..width {
            if code[(x, y)] == Color::Dark {
                let cell_rect = egui::Rect::from_min_size(
                    qr_origin + egui::vec2(x as f32 * scale, y as f32 * scale),
                    egui::vec2(scale, scale),
                );
                painter.rect_filled(cell_rect, 0.0, Color32::from_rgb(18, 22, 28));
            }
        }
    }
}

pub fn export_qr_png(value: &str, path: &Path) -> Result<(), String> {
    use qrcode::types::Color;
    use qrcode::QrCode;
    let code = QrCode::new(value.as_bytes()).map_err(|e| format!("二维码生成失败: {e}"))?;
    let width = code.width();
    let border = 4;
    let scale = 8;
    let total_size = ((width + border * 2) * scale) as u32;
    let mut img = image::RgbaImage::from_pixel(
        total_size,
        total_size,
        image::Rgba([255, 255, 255, 255]),
    );
    for y in 0..width {
        for x in 0..width {
            if code[(x, y)] == Color::Dark {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = ((x + border) * scale + dx) as u32;
                        let py = ((y + border) * scale + dy) as u32;
                        img.put_pixel(px, py, image::Rgba([18, 22, 28, 255]));
                    }
                }
            }
        }
    }
    img.save(path).map_err(|e| format!("保存图片失败: {e}"))?;
    Ok(())
}
