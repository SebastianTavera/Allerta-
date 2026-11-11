//! Página "Reportes": búsqueda, filtros y gestión del estado.

use eframe::egui::{pos2, vec2, Align2, Color32, CursorIcon, Id, Painter, Pos2, Rect, RichText, Rounding, Sense, Ui};

use crate::application::format;
use crate::domain::{Category, Coded, Report, ReportId, Status};
use crate::ui::fx::Fx;
use crate::ui::glass::{self, Glass};
use crate::ui::icons::{self, Glyph};
use crate::ui::motion::Motion;
use crate::ui::theme;
use crate::ui::widgets::{self, flag, ButtonKind, TextField};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserAction {
    ChangeStatus(ReportId, Status),
    RetryNotifications,
}

#[derive(Default)]
pub struct ReportBrowser {
    category: Option<Category>,
    status: Option<Status>,
    query: String,
    expanded: Option<ReportId>,
    /// Cambia para reiniciar la animación escalonada de las tarjetas.
    generation: u64,
}

impl ReportBrowser {
    pub fn restart_animations(&mut self) {
        self.generation += 1;
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        fx: &mut Fx,
        reports: &[Report],
        now: i64,
        mail_enabled: bool,
    ) -> Option<BrowserAction> {
        let mut action = None;
        let active = reports.iter().filter(|r| r.is_active()).count();
        widgets::page_header(
            ui,
            "Reportes",
            &format!("{active} activos · {} resueltos · {} en total", reports.len() - active, reports.len()),
        );

        let pending = reports.iter().filter(|r| !r.notified).count();
        if pending > 0 {
            let text = if mail_enabled {
                format!("{pending} reporte(s) aún no se han notificado por correo.")
            } else {
                format!("{pending} reporte(s) sin notificar: el correo está desactivado. Configura faro.env.")
            };
            widgets::notice(ui, fx, Glyph::Info, &text);
            if mail_enabled && widgets::button(ui, fx, "Reintentar envío", ButtonKind::Secondary).clicked() {
                action = Some(BrowserAction::RetryNotifications);
            }
        }

        TextField::new("search", "Buscar por título, ubicación o descripción…").show(ui, fx, &mut self.query);
        self.filters(ui, fx);
        ui.add_space(6.0);

        let query = self.query.trim().to_lowercase();
        let visible: Vec<&Report> = reports.iter().filter(|r| self.matches(r, &query)).collect();
        if visible.is_empty() {
            let text = if reports.is_empty() {
                "Aún no hay reportes. Gracias por ayudar a cuidar el campus."
            } else {
                "Ningún reporte coincide con los filtros."
            };
            widgets::empty_state(ui, fx, text);
        }
        for (index, report) in visible.into_iter().enumerate() {
            if let Some(card_action) = self.card(ui, fx, report, index, now) {
                action = Some(card_action);
            }
        }
        ui.add_space(40.0);
        action
    }

    fn matches(&self, report: &Report, query: &str) -> bool {
        self.category.map_or(true, |c| report.category == c)
            && self.status.map_or(true, |s| report.status == s)
            && (query.is_empty()
                || [&report.title, &report.location, &report.description]
                    .iter()
                    .any(|text| text.to_lowercase().contains(query)))
    }

    fn filters(&mut self, ui: &mut Ui, fx: &mut Fx) {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            if widgets::pill(ui, fx, "Todas", self.category.is_none()).clicked() {
                self.category = None;
                changed = true;
            }
            for &category in Category::ALL {
                let selected = self.category == Some(category);
                if widgets::pill(ui, fx, category.label(), selected).clicked() {
                    self.category = if selected { None } else { Some(category) };
                    changed = true;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            if widgets::pill(ui, fx, "Todos los estados", self.status.is_none()).clicked() {
                self.status = None;
                changed = true;
            }
            for &status in Status::ALL {
                let selected = self.status == Some(status);
                if widgets::pill(ui, fx, status.label(), selected).clicked() {
                    self.status = if selected { None } else { Some(status) };
                    changed = true;
                }
            }
        });
        if changed {
            self.expanded = None;
            self.restart_animations();
        }
    }

    fn card(&mut self, ui: &mut Ui, fx: &mut Fx, report: &Report, index: usize, now: i64) -> Option<BrowserAction> {
        let id = Id::new(("report-card", report.id.0, self.generation));
        // Cada tarjeta parte un poco "más atrás": entrada escalonada.
        let delay = index.min(10) as f32 * 0.14;
        let appear = fx.springs.enter(id, -delay, 1.0, Motion::Dramatic);
        let expanded = self.expanded == Some(report.id);
        let mut toggled = false;
        let mut action = None;

        widgets::card(ui, fx, appear, |ui, fx| {
            toggled = header(ui, fx, report, now);
            if expanded {
                action = details(ui, fx, id, report);
            } else {
                fx.springs.forget(id.with("reveal"));
            }
        });

        if toggled {
            self.expanded = if expanded { None } else { Some(report.id) };
        }
        action
    }
}

fn header(ui: &mut Ui, fx: &mut Fx, report: &Report, now: i64) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    if !ui.is_rect_visible(rect) {
        return response.clicked();
    }
    let hover = fx.springs.value(response.id.with("hover"), flag(response.hovered()), Motion::Smooth);
    let painter = ui.painter();

    let tile = Rect::from_min_size(rect.min, vec2(46.0, 46.0));
    glass::paint(painter, tile, &Glass::CONTROL.tint(0.05 + 0.05 * hover), fx.light(), 1.0);
    icons::category(painter, tile.center(), 22.0, report.category, theme::TEXT_PRIMARY);

    let right_width = 96.0;
    let text_left = tile.right() + 14.0;
    let text_width = (rect.right() - right_width - text_left).max(40.0);
    widgets::text_line(
        painter,
        pos2(text_left, rect.top() + 3.0),
        &report.title,
        15.5,
        theme::TEXT_PRIMARY,
        text_width,
        Align2::LEFT_TOP,
    );
    let meta = format!(
        "{} · {} · {}",
        report.category.label(),
        report.location,
        format::relative(report.created_at, now)
    );
    widgets::text_line(
        painter,
        pos2(text_left, rect.bottom() - 3.0),
        &meta,
        12.5,
        theme::TEXT_SECONDARY,
        text_width,
        Align2::LEFT_BOTTOM,
    );

    severity_meter(painter, pos2(rect.right(), rect.top() + 6.0), report.severity.level());
    widgets::text_line(
        painter,
        pos2(rect.right(), rect.bottom() - 3.0),
        report.status.label(),
        12.0,
        theme::mix(theme::TEXT_SECONDARY, theme::TEXT_PRIMARY, hover),
        right_width,
        Align2::RIGHT_BOTTOM,
    );
    response.clicked()
}

/// Cuatro barras ascendentes; se rellenan según la gravedad.
fn severity_meter(painter: &Painter, top_right: Pos2, level: u8) {
    for i in 0..4_u8 {
        let height = 6.0 + 3.0 * i as f32;
        let x = top_right.x - (3 - i) as f32 * 7.0 - 4.0;
        let bar = Rect::from_min_max(pos2(x, top_right.y + 15.0 - height), pos2(x + 4.0, top_right.y + 15.0));
        let color = if i < level { Color32::from_gray(232) } else { Color32::from_white_alpha(30) };
        painter.rect_filled(bar, Rounding::same(2.0), color);
    }
}

fn details(ui: &mut Ui, fx: &mut Fx, id: Id, report: &Report) -> Option<BrowserAction> {
    let reveal = fx.springs.enter(id.with("reveal"), 0.0, 1.0, Motion::Dramatic).clamp(0.0, 1.0);
    let mut action = None;
    ui.scope(|ui| {
        ui.multiply_opacity(reveal);
        ui.add_space(6.0 + (1.0 - reveal) * 12.0);
        ui.label(RichText::new(&report.description).size(14.0).color(theme::TEXT_PRIMARY));
        ui.add_space(4.0);
        widgets::detail_row(ui, "Reporte", &report.id.to_string());
        widgets::detail_row(ui, "Fecha", &format::datetime(report.created_at));
        widgets::detail_row(ui, "Ubicación", &report.location);
        widgets::detail_row(ui, "Gravedad", report.severity.label());
        widgets::detail_row(ui, "Reportado por", &report.reporter_display());
        widgets::detail_row(
            ui,
            "Notificación",
            if report.notified { "Correo enviado" } else { "Correo pendiente" },
        );
        ui.add_space(8.0);
        widgets::section_label(ui, "Cambiar estado");
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for &status in Status::ALL {
                let current = report.status == status;
                if widgets::pill(ui, fx, status.label(), current).clicked() && !current {
                    action = Some(BrowserAction::ChangeStatus(report.id, status));
                }
            }
        });
    });
    action
}
