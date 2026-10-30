// Native memo-like workspace: quiet gray canvas, paper surfaces, soft green actions.
#![allow(dead_code)]

use eframe::egui::{self, Color32, Frame, Margin, Rounding, Stroke, Ui};
use std::path::Path;

#[derive(Clone, Copy, PartialEq)]
pub struct ThemeColors {
    pub bg_canvas: Color32,
    pub bg_selection: Color32,
    pub text_on_accent: Color32,
    pub text_accent: Color32,
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

// Semantic color tokens shared by both native appearances.
pub const DARK_THEME: ThemeColors = ThemeColors {
    bg_canvas: Color32::from_rgb(25, 30, 29),
    bg_selection: Color32::from_rgb(39, 61, 52),
    text_on_accent: Color32::from_rgb(17, 42, 31),
    text_accent: Color32::from_rgb(140, 207, 170),
    bg_header: Color32::from_rgb(21, 26, 25),
    bg_card: Color32::from_rgb(30, 36, 34),
    bg_card_elevated: Color32::from_rgb(36, 43, 40),
    bg_card_hover: Color32::from_rgb(44, 53, 49),
    bg_well: Color32::from_rgb(22, 28, 25),
    border_card: Color32::from_rgb(57, 66, 61),
    border_card_light: Color32::from_rgb(90, 105, 97),
    border_accent: Color32::from_rgb(106, 202, 158),
    text_primary: Color32::from_rgb(232, 238, 234),
    text_secondary: Color32::from_rgb(173, 187, 178),
    text_muted: Color32::from_rgb(143, 159, 149),
    accent_emerald: Color32::from_rgb(106, 202, 158),
    accent_emerald_hover: Color32::from_rgb(131, 219, 178),
    accent_emerald_active: Color32::from_rgb(88, 182, 139),
    accent_sky: Color32::from_rgb(140, 188, 218),
    accent_amber: Color32::from_rgb(227, 186, 112),
    accent_rose: Color32::from_rgb(246, 147, 147),
};

pub const LIGHT_THEME: ThemeColors = ThemeColors {
    bg_canvas: Color32::from_rgb(246, 248, 247),
    bg_selection: Color32::from_rgb(222, 239, 229),
    text_on_accent: Color32::from_rgb(22, 49, 35),
    text_accent: Color32::from_rgb(41, 109, 74),
    bg_header: Color32::from_rgb(246, 248, 247),
    bg_card: Color32::from_rgb(253, 254, 253),
    bg_card_elevated: Color32::from_rgb(239, 243, 240),
    bg_card_hover: Color32::from_rgb(232, 239, 235),
    bg_well: Color32::from_rgb(253, 254, 253),
    border_card: Color32::from_rgb(226, 232, 228),
    border_card_light: Color32::from_rgb(167, 181, 173),
    border_accent: Color32::from_rgb(79, 141, 105),
    text_primary: Color32::from_rgb(60, 68, 63),
    text_secondary: Color32::from_rgb(91, 103, 96),
    text_muted: Color32::from_rgb(105, 116, 109),
    accent_emerald: Color32::from_rgb(113, 185, 145),
    accent_emerald_hover: Color32::from_rgb(130, 197, 159),
    accent_emerald_active: Color32::from_rgb(97, 168, 129),
    accent_sky: Color32::from_rgb(62, 105, 142),
    accent_amber: Color32::from_rgb(145, 97, 22),
    accent_rose: Color32::from_rgb(179, 56, 62),
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
    visuals.widgets.inactive.rounding = Rounding::same(5.0);

    visuals.widgets.hovered.bg_fill = t.bg_card_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, t.border_card_light);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, t.text_primary);
    visuals.widgets.hovered.rounding = Rounding::same(5.0);

    visuals.widgets.active.bg_fill = t.accent_emerald_active;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5, t.accent_emerald);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, t.text_primary);
    visuals.widgets.active.weak_bg_fill = t.bg_selection;
    visuals.widgets.hovered.weak_bg_fill = t.bg_card_hover;
    visuals.widgets.inactive.weak_bg_fill = t.bg_well;
    visuals.widgets.noninteractive.weak_bg_fill = t.bg_canvas;
    visuals.widgets.active.rounding = Rounding::same(5.0);

    visuals.selection.bg_fill = t.bg_selection;
    visuals.selection.stroke = Stroke::new(1.0, t.text_primary);

    visuals.window_rounding = Rounding::same(10.0);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = Margin::symmetric(24.0, 16.0);

    use egui::{FontFamily, FontId, TextStyle};
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(20.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Name("Subheading".into()),
            FontId::new(15.5, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        ),
        (
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        ),
    ]
    .into();

    style.spacing.interact_size.y = 30.0;
    style.spacing.text_edit_width = 240.0;
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
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
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

pub fn card_frame(_t: &ThemeColors) -> Frame {
    Frame::none()
}

pub fn paper_frame(t: &ThemeColors) -> Frame {
    Frame::none()
        .fill(t.bg_card)
        .stroke(Stroke::new(1.0, t.border_card))
        .rounding(8.0)
        .inner_margin(Margin::same(14.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: Color32::from_black_alpha(8),
        })
}

pub fn highlighted_card_frame(t: &ThemeColors, _accent_color: Color32) -> Frame {
    paper_frame(t)
}

pub fn section_heading(ui: &mut Ui, title: &str) {
    ui.label(egui::RichText::new(title).size(14.0).strong());
    ui.add_space(4.0);
}

pub fn primary_button<'a>(label: &'a str, t: &ThemeColors) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(label).color(t.text_on_accent).strong())
        .fill(t.accent_emerald)
        .min_size(egui::vec2(112.0, 34.0))
        .rounding(5.0)
}

pub fn focus_outline(ui: &Ui, response: &egui::Response, t: &ThemeColors) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(2.0),
            5.0,
            Stroke::new(1.5, t.border_accent),
        );
    }
}

pub fn segmented_control<T: Copy + PartialEq>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, &str)],
    t: &ThemeColors,
) {
    ui.horizontal(|ui| {
        for (option, label) in options {
            let selected = *value == *option;
            let response = ui.add(
                egui::Button::new(egui::RichText::new(*label).color(if selected {
                    t.text_accent
                } else {
                    t.text_secondary
                }))
                .fill(if selected {
                    t.bg_selection
                } else {
                    Color32::TRANSPARENT
                })
                .stroke(Stroke::NONE)
                .rounding(6.0)
                .min_size(egui::vec2(88.0, 32.0)),
            );
            focus_outline(ui, &response, t);
            if response.clicked() {
                *value = *option;
            }
        }
    });
}

pub fn well_frame(t: &ThemeColors) -> Frame {
    Frame::none()
        .fill(t.bg_well)
        .stroke(Stroke::new(1.0, t.border_card))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(10.0))
}

pub fn pill_badge(ui: &mut Ui, text: &str, color: Color32) {
    let bg_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 20);
    let border_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 70);
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

    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(desired_size, desired_size), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Clean white rounded card background
    painter.rect_filled(rect, Rounding::same(8.0), Color32::from_rgb(255, 255, 255));

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
    let mut img =
        image::RgbaImage::from_pixel(total_size, total_size, image::Rgba([255, 255, 255, 255]));
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
