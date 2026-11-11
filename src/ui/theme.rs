//! Paleta monocromática, medidas y estilo global.

use eframe::egui::{self, Color32, CursorIcon, FontId, Rounding, Stroke, TextStyle, Visuals};

pub const BACKGROUND: Color32 = Color32::from_rgb(9, 9, 11);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(238, 238, 240);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(158, 158, 164);
pub const TEXT_TERTIARY: Color32 = Color32::from_rgb(100, 100, 106);
/// Texto sobre superficies claras.
pub const INK: Color32 = Color32::from_rgb(16, 16, 18);

pub const OUTER_MARGIN: f32 = 16.0;
pub const SIDEBAR_FULL: f32 = 236.0;
pub const SIDEBAR_COMPACT: f32 = 76.0;
pub const COMPACT_BREAKPOINT: f32 = 880.0;
pub const CONTENT_MAX_WIDTH: f32 = 760.0;

pub const RADIUS_PANEL: f32 = 30.0;
pub const RADIUS_CARD: f32 = 22.0;
pub const RADIUS_CONTROL: f32 = 14.0;

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

/// Interpolación lineal entre dos colores.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let channel = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
        channel(a.a(), b.a()),
    )
}

pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = visuals();
    style.text_styles = [
        (TextStyle::Heading, font(28.0)),
        (TextStyle::Body, font(14.5)),
        (TextStyle::Button, font(14.5)),
        (TextStyle::Small, font(11.5)),
        (TextStyle::Monospace, FontId::monospace(12.5)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.scroll = egui::style::ScrollStyle::floating();
    style.interaction.selectable_labels = false;
    ctx.set_style(style);
}

fn visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = BACKGROUND;
    visuals.extreme_bg_color = Color32::TRANSPARENT;
    visuals.window_rounding = Rounding::same(RADIUS_CARD);
    visuals.selection.bg_fill = Color32::from_white_alpha(52);
    visuals.selection.stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.text_cursor.stroke = Stroke::new(1.6_f32, Color32::WHITE);
    visuals.interact_cursor = Some(CursorIcon::PointingHand);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        widget.fg_stroke.color = TEXT_PRIMARY;
    }
    visuals
}
