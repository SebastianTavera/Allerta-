//! Componentes reutilizables de vidrio: tarjetas, botones, campos de texto,
//! controles segmentados, interruptores y chips.

use eframe::egui::text::{LayoutJob, TextWrapping};
use eframe::egui::{
    self, lerp, pos2, vec2, Align, Align2, Color32, CursorIcon, InnerResponse, Layout, Margin,
    Painter, Pos2, Rect, Response, RichText, Rounding, Sense, Shape, TextEdit, Ui,
};

use super::fx::Fx;
use super::glass::{self, unit, Glass};
use super::icons::{self, Glyph};
use super::motion::Motion;
use super::theme;
use crate::domain::{Category, Coded};

pub fn flag(value: bool) -> f32 {
    if value {
        1.0
    } else {
        0.0
    }
}

/// Texto de una línea, recortado con "…" si no cabe.
pub fn text_line(
    painter: &Painter,
    pos: Pos2,
    text: &str,
    size: f32,
    color: Color32,
    max_width: f32,
    anchor: Align2,
) -> Rect {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), theme::font(size), color);
    job.wrap = TextWrapping::truncate_at_width(max_width.max(1.0));
    let galley = painter.layout_job(job);
    let rect = anchor.anchor_size(pos, galley.size());
    painter.galley(rect.min, galley, color);
    rect
}

pub fn page_header(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(30.0).color(theme::TEXT_PRIMARY));
    ui.add_space(-4.0);
    ui.label(RichText::new(subtitle).size(14.0).color(theme::TEXT_SECONDARY));
    ui.add_space(18.0);
}

pub fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(11.0)
            .color(theme::TEXT_TERTIARY)
            .extra_letter_spacing(1.4),
    );
}

/// Contenedor de vidrio que se ajusta a su contenido.
pub fn surface<R>(
    ui: &mut Ui,
    fx: &mut Fx,
    glass: Glass,
    margin: Margin,
    opacity: f32,
    add_contents: impl FnOnce(&mut Ui, &mut Fx) -> R,
) -> InnerResponse<R> {
    let opacity = opacity.clamp(0.0, 1.0);
    let slot = ui.painter().add(Shape::Noop);
    let inner = egui::Frame::none().inner_margin(margin).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.multiply_opacity(opacity);
        add_contents(ui, fx)
    });
    let rect = inner.response.rect;
    if ui.is_rect_visible(rect) {
        ui.painter().set(slot, glass::shape(rect, &glass, fx.light(), opacity));
    }
    inner
}

pub fn card<R>(
    ui: &mut Ui,
    fx: &mut Fx,
    opacity: f32,
    add_contents: impl FnOnce(&mut Ui, &mut Fx) -> R,
) -> R {
    let inner = surface(ui, fx, Glass::CARD, Margin::same(22.0), opacity, add_contents);
    ui.add_space(6.0);
    inner.inner
}

pub fn notice(ui: &mut Ui, fx: &mut Fx, glyph: Glyph, text: &str) {
    let glass = Glass::CARD.tint(0.06).edge(0.24).shadow(0.0);
    surface(ui, fx, glass, Margin::symmetric(18.0, 14.0), 1.0, |ui, _| {
        ui.horizontal(|ui| {
            let (icon, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
            icons::glyph(ui.painter(), icon.center(), 20.0, glyph, theme::TEXT_PRIMARY);
            ui.add(egui::Label::new(RichText::new(text).size(13.5).color(theme::TEXT_PRIMARY)).wrap());
        });
    });
    ui.add_space(6.0);
}

pub fn empty_state(ui: &mut Ui, fx: &mut Fx, text: &str) {
    card(ui, fx, 1.0, |ui, _| {
        ui.vertical_centered(|ui| {
            ui.add_space(18.0);
            let (icon, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
            icons::logo(ui.painter(), icon.center(), 30.0, 0.7);
            ui.add_space(4.0);
            ui.label(RichText::new(text).size(14.0).color(theme::TEXT_SECONDARY));
            ui.add_space(18.0);
        });
    });
}

// ───────────────────────────── Botones ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    /// Vidrio casi opaco y claro, texto oscuro: la acción principal.
    Primary,
    /// Vidrio translúcido.
    Secondary,
}

pub fn button(ui: &mut Ui, fx: &mut Fx, label: &str, kind: ButtonKind) -> Response {
    let text_color = match kind {
        ButtonKind::Primary => theme::INK,
        ButtonKind::Secondary => theme::TEXT_PRIMARY,
    };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), theme::font(14.5), text_color);
    let size = vec2(galley.size().x + 48.0, 44.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let hover = fx.springs.value(response.id.with("hover"), flag(response.hovered()), Motion::Smooth);
        let press = fx.springs.value(
            response.id.with("press"),
            flag(response.is_pointer_button_down_on()),
            Motion::Snappy,
        );
        // Se eleva al pasar el puntero y se hunde al presionar.
        let body = rect.shrink(press * 2.0).translate(vec2(0.0, -1.5 * hover + press));
        let glass = match kind {
            ButtonKind::Primary => Glass::SOLID.tint(0.82 + 0.12 * hover),
            ButtonKind::Secondary => Glass::CONTROL.tint(0.05 + 0.05 * hover).edge(0.16 + 0.16 * hover),
        }
        .radius(body.height() / 2.0);
        glass::paint(ui.painter(), body, &glass, fx.light(), 1.0);
        ui.painter().galley(body.center() - galley.size() / 2.0, galley, text_color);
    }
    response
}

// ─────────────────────────── Campos de texto ───────────────────────────

pub struct TextField<'a> {
    id: &'a str,
    label: Option<&'a str>,
    hint: &'a str,
    multiline: bool,
    error: Option<&'a str>,
}

impl<'a> TextField<'a> {
    pub fn new(id: &'a str, hint: &'a str) -> Self {
        Self { id, label: None, hint, multiline: false, error: None }
    }

    pub fn label(self, label: &'a str) -> Self {
        Self { label: Some(label), ..self }
    }

    pub fn multiline(self) -> Self {
        Self { multiline: true, ..self }
    }

    pub fn error(self, error: Option<&'a str>) -> Self {
        Self { error, ..self }
    }

    pub fn show(self, ui: &mut Ui, fx: &mut Fx, value: &mut String) -> Response {
        if let Some(label) = self.label {
            ui.label(RichText::new(label).size(12.5).color(theme::TEXT_SECONDARY));
            ui.add_space(-4.0);
        }
        let edit_id = ui.make_persistent_id(("field", self.id));
        let focused = ui.memory(|memory| memory.has_focus(edit_id));
        let focus = fx.springs.value(edit_id.with("focus"), flag(focused), Motion::Smooth);
        let failing = fx.springs.value(edit_id.with("error"), flag(self.error.is_some()), Motion::Smooth);

        let slot = ui.painter().add(Shape::Noop);
        let frame = egui::Frame::none().inner_margin(Margin::symmetric(16.0, 12.0)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let edit = if self.multiline {
                TextEdit::multiline(value).desired_rows(4)
            } else {
                TextEdit::singleline(value)
            };
            ui.add(
                edit.id(edit_id)
                    .frame(false)
                    .margin(Margin::ZERO)
                    .desired_width(f32::INFINITY)
                    .font(theme::font(15.0))
                    .text_color(theme::TEXT_PRIMARY)
                    .hint_text(RichText::new(self.hint).color(theme::TEXT_TERTIARY)),
            )
        });
        let rect = frame.response.rect;
        let glass = Glass::CONTROL
            .tint(0.04 + 0.035 * focus)
            .edge(0.12 + 0.3 * focus.max(failing * 0.85));
        ui.painter().set(slot, glass::shape(rect, &glass, fx.light(), 1.0));

        // Clic en cualquier parte del vidrio enfoca el campo.
        if ui.interact(rect, edit_id.with("area"), Sense::click()).clicked() {
            ui.memory_mut(|memory| memory.request_focus(edit_id));
        }
        if let Some(error) = self.error {
            ui.add_space(-4.0);
            ui.label(RichText::new(error).size(12.0).italics().color(theme::TEXT_PRIMARY));
        }
        ui.add_space(4.0);
        frame.inner
    }
}

// ─────────────────────────── Selectores ───────────────────────────

/// Control segmentado con una píldora de vidrio que se desliza y se estira
/// según su velocidad, como una gota de líquido.
pub fn segmented(ui: &mut Ui, fx: &mut Fx, id: &str, options: &[&str], selected: &mut usize) -> bool {
    let id = ui.make_persistent_id(("segmented", id));
    let height = 44.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let inner = rect.shrink(4.0);
    let segment_width = inner.width() / options.len().max(1) as f32;

    let mut changed = false;
    for i in 0..options.len() {
        let segment = Rect::from_min_size(
            pos2(inner.left() + segment_width * i as f32, inner.top()),
            vec2(segment_width, inner.height()),
        );
        let response = ui.interact(segment, id.with(i), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        if response.clicked() && *selected != i {
            *selected = i;
            changed = true;
        }
    }

    let spring = fx.springs.pixels(id.with("pill"), *selected as f32 * segment_width, Motion::Dramatic);
    let stretch = (spring.velocity.abs() * 0.012).min(segment_width * 0.3);
    let pill = Rect::from_min_size(
        pos2(inner.left() + spring.value - stretch * 0.5, inner.top() + stretch * 0.04),
        vec2(segment_width + stretch, inner.height() - stretch * 0.08),
    );

    let painter = ui.painter();
    glass::paint(painter, rect, &Glass::CONTROL.radius(height / 2.0), fx.light(), 1.0);
    glass::paint(painter, pill, &Glass::PILL.radius(inner.height() / 2.0), fx.light(), 1.0);
    for (i, label) in options.iter().enumerate() {
        let center = pos2(inner.left() + segment_width * (i as f32 + 0.5), inner.center().y);
        let closeness = 1.0 - (spring.value / segment_width - i as f32).abs().min(1.0);
        let color = theme::mix(theme::TEXT_SECONDARY, theme::TEXT_PRIMARY, closeness);
        painter.text(center, Align2::CENTER_CENTER, *label, theme::font(13.5), color);
    }
    changed
}

/// Interruptor con perilla elástica.
pub fn toggle(ui: &mut Ui, fx: &mut Fx, label: &str, detail: &str, value: &mut bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    if response.clicked() {
        *value = !*value;
    }
    let t = fx.springs.value(response.id.with("on"), flag(*value), Motion::Dramatic);
    let press = fx.springs.value(
        response.id.with("press"),
        flag(response.is_pointer_button_down_on()),
        Motion::Snappy,
    );
    let on = t.clamp(0.0, 1.0);

    let painter = ui.painter();
    painter.text(rect.left_top() + vec2(0.0, 2.0), Align2::LEFT_TOP, label, theme::font(14.5), theme::TEXT_PRIMARY);
    text_line(
        painter,
        rect.left_bottom() - vec2(0.0, 2.0),
        detail,
        12.5,
        theme::TEXT_SECONDARY,
        rect.width() - 80.0,
        Align2::LEFT_BOTTOM,
    );

    let track = Rect::from_min_size(pos2(rect.right() - 54.0, rect.center().y - 15.0), vec2(54.0, 30.0));
    let track_glass = Glass::CONTROL.radius(15.0).tint(0.05 + 0.8 * on).edge(0.18 + 0.25 * on);
    glass::paint(painter, track, &track_glass, fx.light(), 1.0);

    let knob_width = 24.0 + 7.0 * press;
    let travel = track.width() - 6.0 - knob_width;
    let knob_left = lerp(track.left() + 3.0..=track.left() + 3.0 + travel, t);
    let knob = Rect::from_min_size(pos2(knob_left, track.center().y - 12.0), vec2(knob_width, 24.0));
    painter.rect_filled(knob, Rounding::same(12.0), theme::mix(Color32::from_gray(232), theme::INK, on));
    response
}

/// Mosaico de categoría para el formulario.
pub fn category_tile(ui: &mut Ui, fx: &mut Fx, rect: Rect, category: Category, selected: bool) -> Response {
    let id = ui.make_persistent_id(("category", category.code()));
    let response = ui.interact(rect, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let hover = fx.springs.value(id.with("hover"), flag(response.hovered()), Motion::Smooth);
    let chosen = fx.springs.value(id.with("selected"), flag(selected), Motion::Dramatic);
    let press = fx.springs.value(id.with("press"), flag(response.is_pointer_button_down_on()), Motion::Snappy);

    let body = rect.shrink(press * 2.5);
    let glass = Glass::CARD
        .radius(18.0)
        .shadow(0.0)
        .tint(0.035 + 0.03 * hover + 0.09 * chosen)
        .edge(0.10 + 0.1 * hover + 0.3 * chosen);
    let painter = ui.painter();
    glass::paint(painter, body, &glass, fx.light(), 1.0);

    let color = theme::mix(theme::TEXT_SECONDARY, theme::TEXT_PRIMARY, hover * 0.5 + chosen);
    icons::category(painter, body.left_top() + vec2(28.0, 30.0), 22.0, category, color);
    text_line(
        painter,
        body.left_bottom() + vec2(16.0, -14.0),
        category.label(),
        13.5,
        color,
        body.width() - 32.0,
        Align2::LEFT_BOTTOM,
    );
    if chosen > 0.01 {
        painter.circle_filled(
            body.right_top() + vec2(-18.0, 18.0),
            3.5 * chosen.max(0.0),
            Color32::from_white_alpha(unit(0.9 * chosen)),
        );
    }
    response
}

/// Chip de filtro: translúcido o, si está activo, vidrio claro sólido.
pub fn pill(ui: &mut Ui, fx: &mut Fx, label: &str, selected: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(label.to_owned(), theme::font(13.0), theme::TEXT_PRIMARY);
    let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + 30.0, 34.0), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    if ui.is_rect_visible(rect) {
        let hover = fx.springs.value(response.id.with("hover"), flag(response.hovered()), Motion::Smooth);
        let chosen = fx.springs.value(response.id.with("selected"), flag(selected), Motion::Dramatic);
        let glass = Glass::CONTROL
            .radius(17.0)
            .tint(0.03 + 0.04 * hover + 0.8 * chosen.clamp(0.0, 1.0))
            .edge(0.12 + 0.12 * hover + 0.3 * chosen);
        glass::paint(ui.painter(), rect, &glass, fx.light(), 1.0);
        let base = theme::mix(theme::TEXT_SECONDARY, theme::TEXT_PRIMARY, hover);
        let color = theme::mix(base, theme::INK, chosen);
        ui.painter()
            .galley_with_override_text_color(rect.center() - galley.size() / 2.0, galley, color);
    }
    response
}

/// Fila "etiqueta — valor" para detalles.
pub fn detail_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(vec2(124.0, 18.0), Layout::left_to_right(Align::Center), |ui| {
            ui.label(RichText::new(label).size(12.5).color(theme::TEXT_TERTIARY));
        });
        ui.add(egui::Label::new(RichText::new(value).size(13.0).color(theme::TEXT_PRIMARY)).wrap());
    });
}
