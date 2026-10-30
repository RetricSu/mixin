// Quiet desktop workspace: tinted sidebar, white surfaces, one green accent.
#![allow(dead_code)]

use eframe::egui::{self, Color32, Frame, Margin, Pos2, Rect, Rounding, Stroke, Ui, Vec2};
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
    bg_canvas: Color32::from_rgb(24, 27, 26),
    bg_selection: Color32::from_rgb(35, 56, 46),
    text_on_accent: Color32::from_rgb(255, 255, 255),
    text_accent: Color32::from_rgb(118, 208, 158),
    bg_header: Color32::from_rgb(19, 22, 21),
    bg_card: Color32::from_rgb(31, 35, 33),
    bg_card_elevated: Color32::from_rgb(38, 43, 41),
    bg_card_hover: Color32::from_rgb(44, 50, 47),
    bg_well: Color32::from_rgb(26, 30, 28),
    border_card: Color32::from_rgb(48, 55, 52),
    border_card_light: Color32::from_rgb(66, 76, 71),
    border_accent: Color32::from_rgb(92, 190, 138),
    text_primary: Color32::from_rgb(234, 239, 236),
    text_secondary: Color32::from_rgb(178, 188, 182),
    text_muted: Color32::from_rgb(156, 168, 161),
    accent_emerald: Color32::from_rgb(30, 132, 88),
    accent_emerald_hover: Color32::from_rgb(36, 145, 98),
    accent_emerald_active: Color32::from_rgb(26, 118, 78),
    accent_sky: Color32::from_rgb(140, 188, 218),
    accent_amber: Color32::from_rgb(227, 186, 112),
    accent_rose: Color32::from_rgb(244, 140, 140),
};

pub const LIGHT_THEME: ThemeColors = ThemeColors {
    bg_canvas: Color32::from_rgb(248, 249, 248),
    bg_selection: Color32::from_rgb(225, 241, 232),
    text_on_accent: Color32::from_rgb(255, 255, 255),
    text_accent: Color32::from_rgb(24, 112, 74),
    bg_header: Color32::from_rgb(241, 243, 242),
    bg_card: Color32::from_rgb(255, 255, 255),
    bg_card_elevated: Color32::from_rgb(236, 240, 238),
    bg_card_hover: Color32::from_rgb(232, 237, 234),
    bg_well: Color32::from_rgb(251, 252, 251),
    border_card: Color32::from_rgb(226, 231, 228),
    border_card_light: Color32::from_rgb(204, 212, 207),
    border_accent: Color32::from_rgb(52, 150, 105),
    text_primary: Color32::from_rgb(28, 36, 32),
    text_secondary: Color32::from_rgb(78, 90, 84),
    text_muted: Color32::from_rgb(100, 111, 104),
    accent_emerald: Color32::from_rgb(30, 132, 88),
    accent_emerald_hover: Color32::from_rgb(25, 120, 79),
    accent_emerald_active: Color32::from_rgb(21, 106, 70),
    accent_sky: Color32::from_rgb(52, 98, 138),
    accent_amber: Color32::from_rgb(140, 92, 18),
    accent_rose: Color32::from_rgb(180, 50, 58),
};

pub const RADIUS: f32 = 6.0;
pub const CARD_RADIUS: f32 = 10.0;
pub const PRIMARY_HEIGHT: f32 = 34.0;
pub const CONTROL_HEIGHT: f32 = 30.0;

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
    visuals.window_stroke = Stroke::new(1.0, t.border_card);
    visuals.extreme_bg_color = t.bg_well;
    visuals.faint_bg_color = t.bg_header;
    visuals.code_bg_color = t.bg_well;
    visuals.hyperlink_color = t.text_accent;

    let rounding = Rounding::same(RADIUS);
    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill = t.bg_card;
    w.noninteractive.weak_bg_fill = t.bg_canvas;
    w.noninteractive.bg_stroke = Stroke::new(1.0, t.border_card);
    w.noninteractive.fg_stroke = Stroke::new(1.0, t.text_primary);
    w.noninteractive.rounding = rounding;

    // Secondary buttons and inputs: white surface with a quiet hairline.
    w.inactive.bg_fill = t.bg_well;
    w.inactive.weak_bg_fill = t.bg_card;
    w.inactive.bg_stroke = Stroke::new(1.0, t.border_card_light);
    w.inactive.fg_stroke = Stroke::new(1.5, t.text_secondary);
    w.inactive.rounding = rounding;
    w.inactive.expansion = 0.0;

    w.hovered.bg_fill = t.bg_well;
    w.hovered.weak_bg_fill = t.bg_card_hover;
    w.hovered.bg_stroke = Stroke::new(1.0, t.border_card_light);
    w.hovered.fg_stroke = Stroke::new(1.5, t.text_primary);
    w.hovered.rounding = rounding;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = t.bg_well;
    w.active.weak_bg_fill = t.bg_card_elevated;
    w.active.bg_stroke = Stroke::new(1.0, t.border_accent);
    w.active.fg_stroke = Stroke::new(1.5, t.text_primary);
    w.active.rounding = rounding;
    w.active.expansion = 0.0;

    w.open = w.active;

    visuals.selection.bg_fill = t.bg_selection;
    visuals.selection.stroke = Stroke::new(1.0, t.border_accent);

    visuals.window_rounding = Rounding::same(CARD_RADIUS);
    visuals.menu_rounding = Rounding::same(8.0);
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 16.0,
        spread: 0.0,
        color: Color32::from_black_alpha(if dark_mode { 90 } else { 22 }),
    };
    visuals.text_cursor.stroke = Stroke::new(1.5, t.text_accent);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 5.0);
    style.spacing.window_margin = Margin::symmetric(24.0, 16.0);
    style.spacing.icon_width = 15.0;
    style.spacing.icon_width_inner = 9.0;
    style.spacing.icon_spacing = 6.0;

    use egui::{FontFamily, FontId, TextStyle};
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(20.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Name("Subheading".into()),
            FontId::new(15.0, FontFamily::Proportional),
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

    style.spacing.interact_size.y = CONTROL_HEIGHT;
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

// ---------------------------------------------------------------------------
// Surfaces
// ---------------------------------------------------------------------------

/// The single surface style used for every content group.
pub fn card_frame(t: &ThemeColors) -> Frame {
    Frame::none()
        .fill(t.bg_card)
        .stroke(Stroke::new(1.0, t.border_card))
        .rounding(CARD_RADIUS)
        .inner_margin(Margin::same(16.0))
}

/// Editor surface: same as a card, with a soft lift so it reads as the focus.
pub fn paper_frame(t: &ThemeColors) -> Frame {
    card_frame(t).inner_margin(Margin::symmetric(16.0, 14.0)).shadow(egui::epaint::Shadow {
        offset: egui::vec2(0.0, 1.0),
        blur: 4.0,
        spread: 0.0,
        color: Color32::from_black_alpha(6),
    })
}

/// Inset area inside a card (fingerprints, code blocks).
pub fn well_frame(t: &ThemeColors) -> Frame {
    Frame::none()
        .fill(t.bg_card_elevated)
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::symmetric(12.0, 10.0))
}

/// Full-width card. Every group calls this so no surface collapses to its content width.
pub fn card<R>(ui: &mut Ui, t: &ThemeColors, add: impl FnOnce(&mut Ui) -> R) -> R {
    card_frame(t)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

pub fn paper<R>(ui: &mut Ui, t: &ThemeColors, add: impl FnOnce(&mut Ui) -> R) -> R {
    paper_frame(t)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Thin horizontal rule inside cards.
pub fn hairline(ui: &mut Ui, t: &ThemeColors) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, t.border_card));
}

// ---------------------------------------------------------------------------
// Typography helpers
// ---------------------------------------------------------------------------

/// Group title with an optional trailing control aligned right on the same baseline.
pub fn section_header(ui: &mut Ui, title: &str, t: &ThemeColors, trailing: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_height(CONTROL_HEIGHT);
        ui.label(
            egui::RichText::new(title)
                .size(13.0)
                .strong()
                .color(t.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), trailing);
    });
    ui.add_space(2.0);
}

pub fn section_heading(ui: &mut Ui, title: &str) {
    ui.label(egui::RichText::new(title).size(13.0).strong());
    ui.add_space(4.0);
}

pub fn field_label(ui: &mut Ui, text: &str, t: &ThemeColors) {
    ui.label(egui::RichText::new(text).size(12.5).color(t.text_secondary));
}

pub fn caption(ui: &mut Ui, text: impl Into<String>, t: &ThemeColors) -> egui::Response {
    ui.label(egui::RichText::new(text.into()).size(12.0).color(t.text_muted))
}

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

fn scoped_button(
    ui: &mut Ui,
    label: &str,
    fills: [Color32; 3],
    stroke: Stroke,
    text: Color32,
    min_size: Vec2,
    strong: bool,
) -> egui::Response {
    ui.scope(|ui| {
        let v = ui.visuals_mut();
        v.override_text_color = Some(text);
        for (state, fill) in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
        ]
        .into_iter()
        .zip(fills)
        {
            state.weak_bg_fill = fill;
            state.bg_stroke = stroke;
            state.rounding = Rounding::same(RADIUS);
            state.expansion = 0.0;
        }
        let mut rich = egui::RichText::new(label);
        if strong {
            rich = rich.strong();
        }
        ui.add(egui::Button::new(rich).min_size(min_size))
    })
    .inner
}

/// Filled green call to action. One per view.
pub fn primary_button(ui: &mut Ui, label: &str, t: &ThemeColors) -> egui::Response {
    let response = scoped_button(
        ui,
        label,
        [t.accent_emerald, t.accent_emerald_hover, t.accent_emerald_active],
        Stroke::NONE,
        t.text_on_accent,
        egui::vec2(96.0, PRIMARY_HEIGHT),
        true,
    );
    focus_outline(ui, &response, t);
    response
}

/// Bordered neutral button for secondary actions.
pub fn secondary_button(ui: &mut Ui, label: &str, t: &ThemeColors) -> egui::Response {
    let response = scoped_button(
        ui,
        label,
        [t.bg_card, t.bg_card_hover, t.bg_card_elevated],
        Stroke::new(1.0, t.border_card_light),
        t.text_primary,
        egui::vec2(0.0, CONTROL_HEIGHT),
        false,
    );
    focus_outline(ui, &response, t);
    response
}

/// Borderless text action that only shows a surface on hover.
pub fn ghost_button(ui: &mut Ui, label: &str, t: &ThemeColors) -> egui::Response {
    ghost_button_colored(ui, label, t.text_secondary, t)
}

pub fn ghost_button_colored(
    ui: &mut Ui,
    label: &str,
    color: Color32,
    t: &ThemeColors,
) -> egui::Response {
    let response = ui
        .scope(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);
            scoped_button(
                ui,
                label,
                [Color32::TRANSPARENT, t.bg_card_hover, t.bg_card_elevated],
                Stroke::NONE,
                color,
                egui::vec2(0.0, 28.0),
                false,
            )
        })
        .inner;
    focus_outline(ui, &response, t);
    response
}

/// Destructive confirmation: rose fill with white text.
pub fn danger_button(ui: &mut Ui, label: &str, t: &ThemeColors) -> egui::Response {
    let response = scoped_button(
        ui,
        label,
        [t.accent_rose, t.accent_rose, t.accent_rose],
        Stroke::NONE,
        Color32::WHITE,
        egui::vec2(0.0, CONTROL_HEIGHT),
        true,
    );
    focus_outline(ui, &response, t);
    response
}

pub fn focus_outline(ui: &Ui, response: &egui::Response, t: &ThemeColors) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(2.0),
            RADIUS + 2.0,
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
    Frame::none()
        .fill(t.bg_card_elevated)
        .rounding(RADIUS + 1.0)
        .inner_margin(Margin::same(2.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                for (option, label) in options {
                    let selected = *value == *option;
                    let response = ui
                        .scope(|ui| {
                            let v = ui.visuals_mut();
                            v.override_text_color =
                                Some(if selected { t.text_primary } else { t.text_secondary });
                            for (i, state) in [
                                &mut v.widgets.inactive,
                                &mut v.widgets.hovered,
                                &mut v.widgets.active,
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                state.weak_bg_fill = if selected {
                                    t.bg_card
                                } else if i == 0 {
                                    Color32::TRANSPARENT
                                } else {
                                    t.bg_card_hover
                                };
                                state.bg_stroke = if selected {
                                    Stroke::new(1.0, t.border_card)
                                } else {
                                    Stroke::NONE
                                };
                                state.rounding = Rounding::same(RADIUS - 1.0);
                                state.expansion = 0.0;
                            }
                            let text = egui::RichText::new(*label).size(13.0);
                            ui.add(
                                egui::Button::new(if selected { text.strong() } else { text })
                                    .min_size(egui::vec2(76.0, 26.0)),
                            )
                        })
                        .inner;
                    focus_outline(ui, &response, t);
                    if response.clicked() {
                        *value = *option;
                    }
                }
            });
        });
}

/// Small coloured dot followed by text; used for detection and validation status.
pub fn status_label(ui: &mut Ui, text: &str, color: Color32) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(7.0, 14.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 3.5, color);
        ui.label(egui::RichText::new(text).size(12.5).color(color));
    })
    .response
}

pub fn pill_badge(ui: &mut Ui, text: &str, color: Color32) {
    let bg_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 24);
    Frame::none()
        .fill(bg_color)
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::symmetric(8.0, 2.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(color).size(11.5).strong());
        });
}

// ---------------------------------------------------------------------------
// Icons (vector, drawn in a 16×16 box so they stay crisp at any scale)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    Send,
    Receive,
    Identity,
    Check,
    File,
    Upload,
    Sun,
    Moon,
    Close,
    Info,
    Warning,
    Error,
    Lock,
}

pub fn icon(ui: &mut Ui, icon: Icon, size: f32, color: Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    paint_icon(ui.painter(), rect, icon, color);
    response
}

pub fn paint_icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width() / 16.0;
    let p = |x: f32, y: f32| Pos2::new(rect.min.x + x * s, rect.min.y + y * s);
    let stroke = Stroke::new((1.4 * s).max(1.0), color);
    let line = |pts: &[Pos2]| {
        painter.add(egui::Shape::line(pts.to_vec(), stroke));
    };
    match icon {
        Icon::Send => {
            // Paper plane.
            line(&[p(2.0, 7.5), p(14.0, 2.0), p(9.5, 14.0), p(7.5, 9.0), p(2.0, 7.5)]);
            line(&[p(7.5, 9.0), p(14.0, 2.0)]);
        }
        Icon::Receive => {
            // Tray with an arrow dropping in.
            line(&[p(2.0, 9.5), p(2.0, 13.5), p(14.0, 13.5), p(14.0, 9.5)]);
            line(&[p(2.0, 9.5), p(5.5, 9.5), p(6.5, 11.0), p(9.5, 11.0), p(10.5, 9.5), p(14.0, 9.5)]);
            line(&[p(8.0, 2.0), p(8.0, 8.0)]);
            line(&[p(5.5, 5.5), p(8.0, 8.0), p(10.5, 5.5)]);
        }
        Icon::Identity => {
            painter.circle_stroke(p(8.0, 5.5), 2.8 * s, stroke);
            let mut arc = Vec::new();
            for i in 0..=16 {
                let a = std::f32::consts::PI * (i as f32 / 16.0);
                arc.push(p(8.0 - 5.5 * a.cos(), 14.5 - 4.5 * a.sin()));
            }
            line(&arc);
        }
        Icon::Check => {
            painter.circle_filled(rect.center(), 8.0 * s, color);
            painter.add(egui::Shape::line(
                vec![p(4.8, 8.2), p(7.1, 10.5), p(11.4, 5.8)],
                Stroke::new((1.7 * s).max(1.0), Color32::WHITE),
            ));
        }
        Icon::File => {
            line(&[p(3.5, 1.5), p(9.5, 1.5), p(12.5, 4.5), p(12.5, 14.5), p(3.5, 14.5), p(3.5, 1.5)]);
            line(&[p(9.5, 1.5), p(9.5, 4.5), p(12.5, 4.5)]);
        }
        Icon::Upload => {
            line(&[p(2.0, 10.0), p(2.0, 14.0), p(14.0, 14.0), p(14.0, 10.0)]);
            line(&[p(8.0, 2.0), p(8.0, 10.5)]);
            line(&[p(4.5, 5.5), p(8.0, 2.0), p(11.5, 5.5)]);
        }
        Icon::Sun => {
            painter.circle_stroke(p(8.0, 8.0), 3.0 * s, stroke);
            for i in 0..8 {
                let a = std::f32::consts::TAU * i as f32 / 8.0;
                let (sin, cos) = a.sin_cos();
                line(&[p(8.0 + cos * 5.2, 8.0 + sin * 5.2), p(8.0 + cos * 6.8, 8.0 + sin * 6.8)]);
            }
        }
        Icon::Moon => {
            // Crescent: outer arc of one circle closed by the inner arc of an offset circle.
            let mut pts = Vec::new();
            for i in 0..=24 {
                let a = 0.168 + (4.544 - 0.168) * i as f32 / 24.0;
                pts.push(p(8.0 + 5.5 * a.cos(), 8.0 + 5.5 * a.sin()));
            }
            for i in 0..=16 {
                let a = 3.848 - (3.848 - 0.864) * i as f32 / 16.0;
                pts.push(p(10.5 + 4.5 * a.cos(), 5.5 + 4.5 * a.sin()));
            }
            line(&pts);
        }
        Icon::Close => {
            line(&[p(4.0, 4.0), p(12.0, 12.0)]);
            line(&[p(12.0, 4.0), p(4.0, 12.0)]);
        }
        Icon::Info | Icon::Warning | Icon::Error => {
            painter.circle_filled(rect.center(), 7.0 * s, color);
            let mark = Stroke::new((1.6 * s).max(1.0), Color32::WHITE);
            if icon == Icon::Info {
                painter.circle_filled(p(8.0, 4.9), 1.0 * s, Color32::WHITE);
                painter.line_segment([p(8.0, 7.2), p(8.0, 11.5)], mark);
            } else if icon == Icon::Warning {
                painter.line_segment([p(8.0, 4.2), p(8.0, 8.8)], mark);
                painter.circle_filled(p(8.0, 11.3), 1.0 * s, Color32::WHITE);
            } else {
                painter.line_segment([p(5.6, 5.6), p(10.4, 10.4)], mark);
                painter.line_segment([p(10.4, 5.6), p(5.6, 10.4)], mark);
            }
        }
        Icon::Lock => {
            painter.rect_stroke(
                Rect::from_min_max(p(3.0, 7.0), p(13.0, 14.5)),
                Rounding::same(1.5 * s),
                stroke,
            );
            let mut arc = Vec::new();
            for i in 0..=12 {
                let a = std::f32::consts::PI * (i as f32 / 12.0);
                arc.push(p(8.0 - 3.0 * a.cos(), 7.0 - 3.5 * a.sin()));
            }
            line(&arc);
        }
    }
}

/// Sidebar row: icon + label, tinted when selected, hover surface drawn beneath the text.
pub fn nav_item(ui: &mut Ui, icon: Icon, label: &str, selected: bool, t: &ThemeColors) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, label)
    });
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let fill = if selected {
            t.bg_selection
        } else if response.hovered() {
            t.bg_card_hover
        } else {
            Color32::TRANSPARENT
        };
        painter.rect_filled(rect, RADIUS, fill);
        let color = if selected { t.text_accent } else { t.text_secondary };
        let icon_rect = Rect::from_center_size(
            Pos2::new(rect.left() + 20.0, rect.center().y),
            egui::vec2(16.0, 16.0),
        );
        paint_icon(painter, icon_rect, icon, color);
        let font = egui::FontId::proportional(13.5);
        let galley = painter.layout_no_wrap(label.to_owned(), font, color);
        let pos = Pos2::new(rect.left() + 38.0, rect.center().y - galley.size().y / 2.0);
        if selected {
            // Faux-bold for the selected item keeps metrics identical to the regular weight.
            painter.galley(pos + egui::vec2(0.35, 0.0), galley.clone(), color);
        }
        painter.galley(pos, galley, color);
    }
    focus_outline(ui, &response, t);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ---------------------------------------------------------------------------
// QR
// ---------------------------------------------------------------------------

pub fn render_qr(ui: &mut Ui, value: &str, desired_size: f32) {
    use qrcode::types::Color;
    use qrcode::QrCode;
    let Ok(code) = QrCode::new(value.as_bytes()) else {
        ui.label("无法生成二维码");
        return;
    };
    let width = code.width();
    let margin = 10.0;
    let qr_drawing_size = desired_size - margin * 2.0;
    let scale = qr_drawing_size / width as f32;

    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(desired_size, desired_size), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, Rounding::same(8.0), Color32::WHITE);

    let qr_origin = rect.min + egui::vec2(margin, margin);
    for y in 0..width {
        for x in 0..width {
            if code[(x, y)] == Color::Dark {
                let cell_rect = Rect::from_min_size(
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
