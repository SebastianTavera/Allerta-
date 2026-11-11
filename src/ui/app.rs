//! Ventana principal: fondo, panel lateral de vidrio, páginas y notificaciones.

use eframe::egui::{self, pos2, vec2, Align2, Color32, CursorIcon, Id, Rect, Sense, Stroke, Ui, UiBuilder};

use super::fx::Fx;
use super::glass::{self, unit, Glass};
use super::icons::{self, Glyph};
use super::motion::Motion;
use super::pages::report_browser::{BrowserAction, ReportBrowser};
use super::pages::report_form::{FormAction, ReportForm};
use super::pages::summary;
use super::theme;
use super::toast::{Tone, Toasts};
use super::widgets::{self, flag};
use crate::application::{NotificationEvent, ReportService, ServiceError, Summary};
use crate::domain::{Coded, Report};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Report,
    Browse,
    Overview,
}

impl Page {
    const ALL: [Page; 3] = [Page::Report, Page::Browse, Page::Overview];

    fn label(self) -> &'static str {
        match self {
            Self::Report => "Nuevo reporte",
            Self::Browse => "Reportes",
            Self::Overview => "Resumen",
        }
    }

    fn glyph(self) -> Glyph {
        match self {
            Self::Report => Glyph::Compose,
            Self::Browse => Glyph::List,
            Self::Overview => Glyph::Chart,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Report => "page-report",
            Self::Browse => "page-browse",
            Self::Overview => "page-overview",
        }
    }
}

enum Outcome {
    Idle,
    Form(FormAction),
    Browser(BrowserAction),
}

pub struct FaroApp {
    service: ReportService,
    reports: Vec<Report>,
    summary: Summary,
    page: Page,
    /// Se incrementa al cambiar de página para relanzar las transiciones.
    generation: u64,
    fx: Fx,
    form: ReportForm,
    browser: ReportBrowser,
    toasts: Toasts,
    time: f64,
    now: i64,
}

impl FaroApp {
    pub fn new(cc: &eframe::CreationContext<'_>, service: ReportService) -> Self {
        theme::install(&cc.egui_ctx);
        let mut app = Self {
            service,
            reports: Vec::new(),
            summary: Summary::default(),
            page: Page::Report,
            generation: 0,
            fx: Fx::default(),
            form: ReportForm::default(),
            browser: ReportBrowser::default(),
            toasts: Toasts::default(),
            time: 0.0,
            now: 0,
        };
        app.reload();
        if app.service.notifier().enabled {
            if let Err(err) = app.service.retry_pending() {
                app.toasts.push(Tone::Warning, "No se pudo reintentar el envío", err.to_string(), 0.0);
            }
        }
        app
    }

    fn reload(&mut self) {
        match self.service.reports() {
            Ok(reports) => {
                self.summary = Summary::from_reports(&reports);
                self.reports = reports;
            }
            Err(err) => self.toasts.push(Tone::Warning, "No se pudieron cargar los reportes", err.to_string(), self.time),
        }
    }

    fn navigate(&mut self, page: Page) {
        if page != self.page {
            self.page = page;
            self.generation += 1;
            self.browser.restart_animations();
        }
    }

    // ─────────────────────────── Casos de uso ───────────────────────────

    fn submit_report(&mut self) {
        match self.service.submit(self.form.draft()) {
            Ok(report) => {
                self.form.clear();
                let notifier = self.service.notifier();
                let body = if notifier.enabled {
                    format!("Notificando a {}…", notifier.destination)
                } else {
                    "Guardado. El correo se enviará al configurar SMTP.".to_owned()
                };
                self.toasts.push(Tone::Success, format!("Reporte {} registrado", report.id), body, self.time);
                self.reload();
            }
            Err(ServiceError::Invalid(errors)) => {
                self.form.errors = errors;
                self.toasts.push(Tone::Info, "Revisa el formulario", "Hay campos por completar.", self.time);
            }
            Err(err) => self.toasts.push(Tone::Warning, "No se pudo guardar", err.to_string(), self.time),
        }
    }

    fn apply_browser_action(&mut self, action: BrowserAction) {
        match action {
            BrowserAction::ChangeStatus(id, status) => match self.service.change_status(id, status) {
                Ok(()) => {
                    self.toasts.push(Tone::Info, "Estado actualizado", format!("{id} → {}", status.label()), self.time);
                    self.reload();
                }
                Err(err) => self.toasts.push(Tone::Warning, "No se pudo actualizar", err.to_string(), self.time),
            },
            BrowserAction::RetryNotifications => match self.service.retry_pending() {
                Ok(count) => self.toasts.push(Tone::Info, "Reintentando envío", format!("{count} reporte(s) en cola"), self.time),
                Err(err) => self.toasts.push(Tone::Warning, "No se pudo reintentar", err.to_string(), self.time),
            },
        }
    }

    fn handle_notifications(&mut self) {
        let events = self.service.poll_notifications();
        if events.is_empty() {
            return;
        }
        let delivered = events.iter().filter(|e| matches!(e, NotificationEvent::Delivered(_))).count();
        let failure = events.iter().find_map(|event| match event {
            NotificationEvent::Failed { reason, .. } => Some(reason.clone()),
            NotificationEvent::Delivered(_) => None,
        });
        if delivered > 0 {
            let destination = &self.service.notifier().destination;
            let body = format!("{delivered} notificación(es) enviada(s) a {destination}");
            self.toasts.push(Tone::Success, "Correo enviado", body, self.time);
        }
        if let Some(reason) = failure {
            self.toasts.push(Tone::Warning, "Correo pendiente", reason, self.time);
        }
        self.reload();
    }

    // ───────────────────────────── Diseño ─────────────────────────────

    fn layout(&mut self, ui: &mut Ui) {
        let screen = ui.max_rect();
        glass::background(ui.painter(), screen, self.fx.light());

        let margin = theme::OUTER_MARGIN;
        let target_width = if screen.width() < theme::COMPACT_BREAKPOINT {
            theme::SIDEBAR_COMPACT
        } else {
            theme::SIDEBAR_FULL
        };
        let width = self.fx.springs.pixels(Id::new("sidebar-width"), target_width, Motion::Dramatic).value;
        let intro = self.fx.springs.enter(Id::new("intro"), 0.0, 1.0, Motion::Dramatic);

        let sidebar = Rect::from_min_max(
            pos2(screen.left() + margin, screen.top() + margin),
            pos2(screen.left() + margin + width, screen.bottom() - margin),
        )
        .translate(vec2(-(1.0 - intro) * 60.0, 0.0));
        self.sidebar(ui, sidebar, intro.clamp(0.0, 1.0));

        let content = Rect::from_min_max(pos2(screen.left() + margin * 2.0 + width, screen.top()), screen.max);
        self.content(ui, content);
    }

    fn sidebar(&mut self, ui: &mut Ui, rect: Rect, opacity: f32) {
        let light = self.fx.light();
        let painter = ui.painter().clone();
        glass::paint(&painter, rect, &Glass::PANEL, light, opacity);
        let clip = painter.with_clip_rect(rect.shrink(1.0).intersect(painter.clip_rect()));

        // 0 = compacto (solo iconos), 1 = expandido.
        let expand = ((rect.width() - theme::SIDEBAR_COMPACT) / (theme::SIDEBAR_FULL - theme::SIDEBAR_COMPACT)).clamp(0.0, 1.0);
        let label_alpha = opacity * expand * expand;
        let icon_x = rect.left() + theme::SIDEBAR_COMPACT / 2.0;
        let text_x = rect.left() + 66.0;

        icons::logo(&clip, pos2(icon_x, rect.top() + 46.0), 26.0, opacity);
        clip.text(pos2(text_x, rect.top() + 46.0), Align2::LEFT_BOTTOM, "Faro", theme::font(22.0), theme::TEXT_PRIMARY.gamma_multiply(label_alpha));
        clip.text(
            pos2(text_x, rect.top() + 48.0),
            Align2::LEFT_TOP,
            "Seguridad en el campus",
            theme::font(11.5),
            theme::TEXT_TERTIARY.gamma_multiply(label_alpha),
        );

        // Navegación con píldora de vidrio que se desliza y se estira.
        let item_height = 46.0;
        let gap = 6.0;
        let top = rect.top() + 104.0;
        let index = Page::ALL.iter().position(|page| *page == self.page).unwrap_or(0) as f32;
        let pill = self.fx.springs.pixels(Id::new("nav-pill"), top + index * (item_height + gap), Motion::Dramatic);
        let stretch = (pill.velocity.abs() * 0.02).min(26.0);
        let pill_rect = Rect::from_min_max(
            pos2(rect.left() + 10.0 + stretch * 0.1, pill.value - stretch * 0.5),
            pos2(rect.right() - 10.0 - stretch * 0.1, pill.value + item_height + stretch * 0.5),
        );
        glass::paint(&clip, pill_rect, &Glass::PILL.radius(16.0), light, opacity);

        let mut target = None;
        for (i, page) in Page::ALL.iter().enumerate() {
            let item = Rect::from_min_size(
                pos2(rect.left() + 10.0, top + i as f32 * (item_height + gap)),
                vec2(rect.width() - 20.0, item_height),
            );
            let response = ui.interact(item, Id::new(("nav", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            if response.clicked() {
                target = Some(*page);
            }
            let hover = self.fx.springs.value(Id::new(("nav-hover", i)), flag(response.hovered()), Motion::Smooth);
            let tone = if *page == self.page { 1.0 } else { 0.4 + 0.45 * hover };
            let color = theme::mix(theme::TEXT_TERTIARY, theme::TEXT_PRIMARY, tone).gamma_multiply(opacity);
            let y = item.center().y;
            icons::glyph(&clip, pos2(icon_x, y), 20.0, page.glyph(), color);
            clip.text(pos2(text_x, y), Align2::LEFT_CENTER, page.label(), theme::font(14.5), color.gamma_multiply(expand * expand));
            if *page == Page::Browse && self.summary.active() > 0 {
                clip.text(
                    pos2(item.right() - 14.0, y),
                    Align2::RIGHT_CENTER,
                    self.summary.active().to_string(),
                    theme::font(12.0),
                    theme::TEXT_SECONDARY.gamma_multiply(label_alpha),
                );
            }
        }
        if let Some(page) = target {
            self.navigate(page);
        }

        self.sidebar_status(&clip, rect, opacity, expand);
    }

    /// Estado del canal de correo y número de emergencias.
    fn sidebar_status(&self, painter: &egui::Painter, rect: Rect, opacity: f32, expand: f32) {
        let notifier = self.service.notifier();
        let dot_color = Color32::from_white_alpha(unit(0.9 * opacity));
        let dot = |center| {
            if notifier.enabled {
                painter.circle_filled(center, 4.0, dot_color);
            } else {
                painter.circle_stroke(center, 4.0, Stroke::new(1.3_f32, dot_color));
            }
        };

        if expand < 0.98 {
            let alpha = 1.0 - expand;
            painter.circle_stroke(
                pos2(rect.left() + theme::SIDEBAR_COMPACT / 2.0, rect.bottom() - 36.0),
                9.0,
                Stroke::new(1.0_f32, Color32::from_white_alpha(unit(0.25 * alpha * opacity))),
            );
            if alpha > 0.5 {
                dot(pos2(rect.left() + theme::SIDEBAR_COMPACT / 2.0, rect.bottom() - 36.0));
            }
        }
        if expand <= 0.02 {
            return;
        }

        let alpha = opacity * expand * expand;
        let block = Rect::from_min_max(pos2(rect.left() + 12.0, rect.bottom() - 108.0), pos2(rect.right() - 12.0, rect.bottom() - 12.0));
        glass::paint(painter, block, &Glass::CONTROL.radius(18.0), self.fx.light(), alpha);
        let left = block.left() + 16.0;
        let width = block.width() - 32.0;
        if expand > 0.5 {
            dot(pos2(left + 4.0, block.top() + 24.0));
        }
        let state = if notifier.enabled { "Correo activo" } else { "Correo desactivado" };
        widgets::text_line(painter, pos2(left + 16.0, block.top() + 24.0), state, 13.0, theme::TEXT_PRIMARY.gamma_multiply(alpha), width - 16.0, Align2::LEFT_CENTER);
        widgets::text_line(painter, pos2(left, block.top() + 48.0), &notifier.destination, 11.5, theme::TEXT_SECONDARY.gamma_multiply(alpha), width, Align2::LEFT_CENTER);
        widgets::text_line(painter, pos2(left, block.top() + 72.0), "Emergencias · Línea 123", 11.5, theme::TEXT_TERTIARY.gamma_multiply(alpha), width, Align2::LEFT_CENTER);
    }

    fn content(&mut self, ui: &mut Ui, area: Rect) {
        // Transición de página: sube y aparece con inercia.
        let t = self.fx.springs.enter(Id::new(("page", self.generation)), 0.0, 1.0, Motion::Dramatic);
        let rect = area.translate(vec2(0.0, (1.0 - t) * 36.0));
        let mut outcome = Outcome::Idle;

        ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| {
            ui.set_clip_rect(area);
            ui.multiply_opacity(t.clamp(0.0, 1.0));
            egui::ScrollArea::vertical()
                .id_salt(self.page.key())
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let full = ui.available_width();
                    let column = (full - 48.0).clamp(200.0, theme::CONTENT_MAX_WIDTH);
                    let pad = ((full - column) / 2.0).max(0.0);
                    ui.add_space(30.0);
                    ui.horizontal_top(|ui| {
                        ui.add_space(pad);
                        ui.vertical(|ui| {
                            ui.set_width(column);
                            outcome = match self.page {
                                Page::Report => Outcome::Form(self.form.show(ui, &mut self.fx)),
                                Page::Browse => self
                                    .browser
                                    .show(ui, &mut self.fx, &self.reports, self.now, self.service.notifier().enabled)
                                    .map_or(Outcome::Idle, Outcome::Browser),
                                Page::Overview => {
                                    summary::show(ui, &mut self.fx, &self.summary, self.generation);
                                    Outcome::Idle
                                }
                            };
                        });
                    });
                });
        });

        match outcome {
            Outcome::Form(FormAction::Submit) => self.submit_report(),
            Outcome::Form(FormAction::Clear) => self.form.clear(),
            Outcome::Browser(action) => self.apply_browser_action(action),
            Outcome::Form(FormAction::Idle) | Outcome::Idle => {}
        }
    }
}

impl eframe::App for FaroApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::BACKGROUND.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.time = ctx.input(|i| i.time);
        self.now = self.service.now();
        self.fx.begin_frame(ctx);
        self.handle_notifications();

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| self.layout(ui));

        self.toasts.show(ctx, &mut self.fx, self.time);
        self.fx.end_frame(ctx);
    }
}
