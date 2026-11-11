//! Página "Resumen": indicadores y distribución de los reportes.

use eframe::egui::{pos2, vec2, Align2, Color32, Id, Rect, RichText, Rounding, Sense, Ui};

use crate::application::Summary;
use crate::domain::{Category, Coded};
use crate::ui::fx::Fx;
use crate::ui::glass::{self, Glass};
use crate::ui::icons;
use crate::ui::motion::Motion;
use crate::ui::theme;
use crate::ui::widgets;

const BAR_FILL: Color32 = Color32::from_gray(226);

pub fn show(ui: &mut Ui, fx: &mut Fx, summary: &Summary, generation: u64) {
    widgets::page_header(ui, "Resumen", "Panorama en tiempo real de la seguridad del campus.");
    stat_tiles(ui, fx, summary, generation);
    ui.add_space(8.0);

    widgets::card(ui, fx, 1.0, |ui, fx| {
        widgets::section_label(ui, "Por categoría");
        let max = summary.by_category.iter().map(|(_, n)| *n).max().unwrap_or(0).max(1);
        for (index, (category, count)) in summary.by_category.iter().enumerate() {
            bar_row(ui, fx, Id::new(("category-bar", index, generation)), *category, *count, max);
        }
    });

    widgets::card(ui, fx, 1.0, |ui, fx| {
        widgets::section_label(ui, "Por gravedad");
        severity_columns(ui, fx, summary, generation);
    });

    widgets::card(ui, fx, 1.0, |ui, _| {
        widgets::section_label(ui, "Zonas con más reportes activos");
        if summary.hotspots.is_empty() {
            ui.label(RichText::new("Aún no hay datos suficientes.").size(13.5).color(theme::TEXT_SECONDARY));
        }
        for (rank, (location, count)) in summary.hotspots.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{:02}", rank + 1)).size(13.0).color(theme::TEXT_TERTIARY));
                ui.label(RichText::new(location).size(14.0).color(theme::TEXT_PRIMARY));
                ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::Center), |ui| {
                    let noun = if *count == 1 { "reporte" } else { "reportes" };
                    ui.label(RichText::new(format!("{count} {noun}")).size(13.0).color(theme::TEXT_SECONDARY));
                });
            });
        }
    });
    ui.add_space(40.0);
}

fn stat_tiles(ui: &mut Ui, fx: &mut Fx, summary: &Summary, generation: u64) {
    let stats = [
        ("Reportes totales", summary.total),
        ("Activos", summary.active()),
        ("Críticos activos", summary.critical_active),
        ("Resueltos", summary.resolved),
    ];
    let width = ui.available_width();
    let columns = if width > 600.0 { 4 } else { 2 };
    let rows = stats.len().div_ceil(columns);
    let gap = 12.0;
    let tile_width = (width - gap * (columns - 1) as f32) / columns as f32;
    let tile_height = 108.0;
    let (area, _) = ui.allocate_exact_size(
        vec2(width, rows as f32 * tile_height + (rows - 1) as f32 * gap),
        Sense::hover(),
    );

    for (index, (label, value)) in stats.iter().enumerate() {
        let (row, column) = (index / columns, index % columns);
        let rect = Rect::from_min_size(
            pos2(area.left() + column as f32 * (tile_width + gap), area.top() + row as f32 * (tile_height + gap)),
            vec2(tile_width, tile_height),
        );
        let appear = fx.springs.enter(Id::new(("stat-in", index, generation)), -(index as f32) * 0.15, 1.0, Motion::Dramatic);
        let shown = fx.springs.count(Id::new(("stat-value", index, generation)), *value as f32);
        let visible = appear.clamp(0.0, 1.0);
        let body = rect.translate(vec2(0.0, (1.0 - appear) * 18.0));

        let painter = ui.painter();
        glass::paint(painter, body, &Glass::CARD, fx.light(), visible);
        painter.text(
            body.left_top() + vec2(20.0, 16.0),
            Align2::LEFT_TOP,
            format!("{}", shown.round().max(0.0) as i64),
            theme::font(36.0),
            theme::TEXT_PRIMARY.gamma_multiply(visible),
        );
        widgets::text_line(
            painter,
            body.left_bottom() + vec2(20.0, -18.0),
            label,
            12.5,
            theme::TEXT_SECONDARY.gamma_multiply(visible),
            body.width() - 40.0,
            Align2::LEFT_BOTTOM,
        );
    }
}

fn bar_row(ui: &mut Ui, fx: &mut Fx, id: Id, category: Category, count: usize, max: usize) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
    let fraction = fx.springs.enter(id, 0.0, count as f32 / max as f32, Motion::Dramatic);
    let painter = ui.painter();
    let y = rect.center().y;

    icons::category(painter, pos2(rect.left() + 10.0, y), 18.0, category, theme::TEXT_SECONDARY);
    let label_left = rect.left() + 32.0;
    let label_width = (rect.width() * 0.36).min(190.0);
    widgets::text_line(
        painter,
        pos2(label_left, y),
        category.label(),
        13.5,
        theme::TEXT_PRIMARY,
        label_width - 10.0,
        Align2::LEFT_CENTER,
    );

    let track = Rect::from_min_max(pos2(label_left + label_width, y - 5.0), pos2(rect.right() - 40.0, y + 5.0));
    glass::paint(painter, track, &Glass::CONTROL.radius(5.0), fx.light(), 1.0);
    if fraction > 0.001 {
        let width = (track.width() * fraction).max(track.height());
        painter.rect_filled(Rect::from_min_size(track.min, vec2(width, track.height())), Rounding::same(5.0), BAR_FILL);
    }
    painter.text(pos2(rect.right(), y), Align2::RIGHT_CENTER, count.to_string(), theme::font(13.5), theme::TEXT_SECONDARY);
}

fn severity_columns(ui: &mut Ui, fx: &mut Fx, summary: &Summary, generation: u64) {
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), 170.0), Sense::hover());
    let max = summary.by_severity.iter().map(|(_, n)| *n).max().unwrap_or(0).max(1);
    let columns = summary.by_severity.len().max(1) as f32;
    let column_width = area.width() / columns;
    let chart_top = area.top() + 24.0;
    let chart_bottom = area.bottom() - 26.0;

    for (index, (severity, count)) in summary.by_severity.iter().enumerate() {
        let center_x = area.left() + column_width * (index as f32 + 0.5);
        let fraction = fx.springs.enter(
            Id::new(("severity-bar", index, generation)),
            -(index as f32) * 0.1,
            *count as f32 / max as f32,
            Motion::Dramatic,
        );
        let painter = ui.painter();
        let track = Rect::from_min_max(pos2(center_x - 13.0, chart_top), pos2(center_x + 13.0, chart_bottom));
        glass::paint(painter, track, &Glass::CONTROL.radius(13.0), fx.light(), 1.0);
        if fraction > 0.001 {
            let height = (track.height() * fraction.min(1.08)).max(track.width());
            let fill = Rect::from_min_max(pos2(track.left(), track.bottom() - height), track.max);
            painter.rect_filled(fill, Rounding::same(13.0), BAR_FILL);
        }
        painter.text(pos2(center_x, area.top() + 8.0), Align2::CENTER_CENTER, count.to_string(), theme::font(13.5), theme::TEXT_PRIMARY);
        painter.text(pos2(center_x, area.bottom() - 8.0), Align2::CENTER_CENTER, severity.label(), theme::font(12.5), theme::TEXT_SECONDARY);
    }
}
