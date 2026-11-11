//! Página "Nuevo reporte".

use eframe::egui::{pos2, vec2, Rect, RichText, Sense, Ui};

use crate::domain::{Category, Coded, Field, NewReport, Reporter, Severity, ValidationErrors};
use crate::ui::fx::Fx;
use crate::ui::icons::Glyph;
use crate::ui::motion::Motion;
use crate::ui::theme;
use crate::ui::widgets::{self, ButtonKind, TextField};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormAction {
    Idle,
    Submit,
    Clear,
}

pub struct ReportForm {
    category: Category,
    severity: usize,
    title: String,
    location: String,
    description: String,
    anonymous: bool,
    name: String,
    email: String,
    pub errors: ValidationErrors,
}

impl Default for ReportForm {
    fn default() -> Self {
        Self {
            category: Category::UnsafeZone,
            severity: 1,
            title: String::new(),
            location: String::new(),
            description: String::new(),
            anonymous: true,
            name: String::new(),
            email: String::new(),
            errors: ValidationErrors::default(),
        }
    }
}

impl ReportForm {
    pub fn draft(&self) -> NewReport {
        NewReport {
            category: self.category,
            severity: Severity::ALL[self.severity.min(Severity::ALL.len() - 1)],
            title: self.title.clone(),
            location: self.location.clone(),
            description: self.description.clone(),
            reporter: (!self.anonymous)
                .then(|| Reporter { name: self.name.clone(), email: Some(self.email.clone()) }),
        }
    }

    /// Limpia el contenido conservando la identidad elegida.
    pub fn clear(&mut self) {
        *self = Self {
            anonymous: self.anonymous,
            name: std::mem::take(&mut self.name),
            email: std::mem::take(&mut self.email),
            ..Self::default()
        };
    }

    pub fn show(&mut self, ui: &mut Ui, fx: &mut Fx) -> FormAction {
        widgets::page_header(
            ui,
            "Reportar una situación",
            "Tu reporte llega de inmediato al equipo de bienestar y seguridad del campus.",
        );

        widgets::card(ui, fx, 1.0, |ui, fx| {
            widgets::section_label(ui, "¿Qué está pasando?");
            self.category_grid(ui, fx);
            ui.label(RichText::new(self.category.hint()).size(13.0).color(theme::TEXT_SECONDARY));
        });

        if self.category.is_urgent() {
            widgets::notice(
                ui,
                fx,
                Glyph::Alert,
                "Si hay peligro inmediato, llama a la Línea de Emergencias 123 o acude al personal de seguridad más cercano.",
            );
        }

        widgets::card(ui, fx, 1.0, |ui, fx| {
            widgets::section_label(ui, "Detalles");
            TextField::new("title", "Ej. Baranda suelta en la escalera principal")
                .label("Título")
                .error(self.errors.get(Field::Title))
                .show(ui, fx, &mut self.title);
            TextField::new("location", "Ej. Bloque C, segundo piso, junto al ascensor")
                .label("Ubicación")
                .error(self.errors.get(Field::Location))
                .show(ui, fx, &mut self.location);
            TextField::new("description", "Cuenta qué ocurre, desde cuándo y si hay personas en riesgo.")
                .label("Descripción")
                .multiline()
                .error(self.errors.get(Field::Description))
                .show(ui, fx, &mut self.description);
            ui.label(RichText::new("Gravedad").size(12.5).color(theme::TEXT_SECONDARY));
            let labels: Vec<&str> = Severity::ALL.iter().map(|severity| severity.label()).collect();
            widgets::segmented(ui, fx, "severity", &labels, &mut self.severity);
        });

        widgets::card(ui, fx, 1.0, |ui, fx| {
            widgets::section_label(ui, "Identidad");
            let detail = if self.anonymous {
                "Tu nombre no se guardará ni se enviará."
            } else {
                "Tu nombre se incluirá para poder hacer seguimiento."
            };
            widgets::toggle(ui, fx, "Reportar de forma anónima", detail, &mut self.anonymous);

            let reveal_id = ui.make_persistent_id("identity-reveal");
            if self.anonymous {
                fx.springs.forget(reveal_id);
            } else {
                let reveal = fx.springs.enter(reveal_id, 0.0, 1.0, Motion::Dramatic).clamp(0.0, 1.0);
                ui.scope(|ui| {
                    ui.multiply_opacity(reveal);
                    ui.add_space(6.0 + (1.0 - reveal) * 14.0);
                    TextField::new("name", "Tu nombre completo")
                        .label("Nombre")
                        .error(self.errors.get(Field::ReporterName))
                        .show(ui, fx, &mut self.name);
                    TextField::new("email", "tu.correo@academia.umb.edu.co (opcional)")
                        .label("Correo para seguimiento")
                        .error(self.errors.get(Field::ReporterEmail))
                        .show(ui, fx, &mut self.email);
                });
            }
        });

        let mut action = FormAction::Idle;
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if widgets::button(ui, fx, "Enviar reporte", ButtonKind::Primary).clicked() {
                action = FormAction::Submit;
            }
            if widgets::button(ui, fx, "Limpiar", ButtonKind::Secondary).clicked() {
                action = FormAction::Clear;
            }
        });
        ui.add_space(40.0);
        action
    }

    fn category_grid(&mut self, ui: &mut Ui, fx: &mut Fx) {
        let width = ui.available_width();
        let columns = if width > 540.0 { 3 } else { 2 };
        let gap = 10.0;
        let tile_width = (width - gap * (columns - 1) as f32) / columns as f32;
        let tile_height = 86.0;
        let rows = Category::ALL.len().div_ceil(columns);
        let (area, _) = ui.allocate_exact_size(
            vec2(width, rows as f32 * tile_height + (rows - 1) as f32 * gap),
            Sense::hover(),
        );

        for (index, &category) in Category::ALL.iter().enumerate() {
            let (row, column) = (index / columns, index % columns);
            let tile = Rect::from_min_size(
                pos2(
                    area.left() + column as f32 * (tile_width + gap),
                    area.top() + row as f32 * (tile_height + gap),
                ),
                vec2(tile_width, tile_height),
            );
            if widgets::category_tile(ui, fx, tile, category, self.category == category).clicked() {
                self.category = category;
            }
        }
    }
}
