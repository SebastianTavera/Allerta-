//! Entidades y reglas de negocio de un reporte de riesgo en el campus.

use std::fmt;
use std::ops::RangeInclusive;

/// Enumeraciones que se persisten mediante un código estable y se muestran
/// con una etiqueta legible. Centraliza la conversión código ⇄ variante.
pub trait Coded: Copy + PartialEq + 'static {
    const ALL: &'static [Self];

    fn code(self) -> &'static str;
    fn label(self) -> &'static str;

    fn from_code(code: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|item| item.code() == code)
    }
}

// ───────────────────────────── Categoría ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    UnsafeZone,
    FacilityDamage,
    Emergency,
    Harassment,
    PoorLighting,
    OtherRisk,
}

impl Coded for Category {
    const ALL: &'static [Self] = &[
        Self::UnsafeZone,
        Self::FacilityDamage,
        Self::Emergency,
        Self::Harassment,
        Self::PoorLighting,
        Self::OtherRisk,
    ];

    fn code(self) -> &'static str {
        match self {
            Self::UnsafeZone => "unsafe_zone",
            Self::FacilityDamage => "facility_damage",
            Self::Emergency => "emergency",
            Self::Harassment => "harassment",
            Self::PoorLighting => "poor_lighting",
            Self::OtherRisk => "other_risk",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::UnsafeZone => "Zona insegura",
            Self::FacilityDamage => "Daño en instalaciones",
            Self::Emergency => "Emergencia",
            Self::Harassment => "Acoso",
            Self::PoorLighting => "Falta de iluminación",
            Self::OtherRisk => "Otro riesgo",
        }
    }
}

impl Category {
    pub fn hint(self) -> &'static str {
        match self {
            Self::UnsafeZone => {
                "Lugares donde no te sientes seguro: robos, personas sospechosas o accesos sin control."
            }
            Self::FacilityDamage => {
                "Escaleras, barandas, cableado, techos o mobiliario en mal estado."
            }
            Self::Emergency => {
                "Situaciones que requieren atención inmediata: incendios, personas heridas, fugas."
            }
            Self::Harassment => {
                "Acoso verbal, físico, sexual o por medios digitales dentro del campus."
            }
            Self::PoorLighting => {
                "Pasillos, parqueaderos o senderos con iluminación insuficiente."
            }
            Self::OtherRisk => "Cualquier otra situación que ponga en riesgo a la comunidad.",
        }
    }

    /// Categorías que exigen atención prioritaria.
    pub fn is_urgent(self) -> bool {
        matches!(self, Self::Emergency | Self::Harassment)
    }
}

// ───────────────────────────── Gravedad ──────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Coded for Severity {
    const ALL: &'static [Self] = &[Self::Low, Self::Medium, Self::High, Self::Critical];

    fn code(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Low => "Baja",
            Self::Medium => "Media",
            Self::High => "Alta",
            Self::Critical => "Crítica",
        }
    }
}

impl Severity {
    /// Nivel de 1 (baja) a 4 (crítica).
    pub fn level(self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }
}

// ────────────────────────────── Estado ───────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    Open,
    InReview,
    Resolved,
}

impl Coded for Status {
    const ALL: &'static [Self] = &[Self::Open, Self::InReview, Self::Resolved];

    fn code(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InReview => "in_review",
            Self::Resolved => "resolved",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Open => "Abierto",
            Self::InReview => "En revisión",
            Self::Resolved => "Resuelto",
        }
    }
}

// ───────────────────────────── Entidades ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReportId(pub i64);

impl fmt::Display for ReportId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:04}", self.0)
    }
}

/// Persona que reporta. `None` en el reporte significa reporte anónimo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reporter {
    pub name: String,
    pub email: Option<String>,
}

/// Campos validables de un reporte, para ubicar los errores en el formulario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Title,
    Location,
    Description,
    ReporterName,
    ReporterEmail,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationErrors {
    items: Vec<(Field, &'static str)>,
}

impl ValidationErrors {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, field: Field) -> Option<&'static str> {
        self.items
            .iter()
            .find(|(candidate, _)| *candidate == field)
            .map(|(_, message)| *message)
    }

    fn push(&mut self, field: Field, message: &'static str) {
        self.items.push((field, message));
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages: Vec<&str> = self.items.iter().map(|(_, message)| *message).collect();
        f.write_str(&messages.join(" "))
    }
}

impl std::error::Error for ValidationErrors {}

/// Reporte aún no persistido, tal como lo envía un estudiante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewReport {
    pub category: Category,
    pub severity: Severity,
    pub title: String,
    pub location: String,
    pub description: String,
    pub reporter: Option<Reporter>,
}

impl NewReport {
    pub const TITLE_LEN: RangeInclusive<usize> = 4..=120;
    pub const LOCATION_LEN: RangeInclusive<usize> = 2..=120;
    pub const DESCRIPTION_LEN: RangeInclusive<usize> = 10..=2000;
    pub const NAME_LEN: RangeInclusive<usize> = 2..=80;

    /// Limpia espacios sobrantes y descarta un correo vacío.
    pub fn normalized(self) -> Self {
        Self {
            title: self.title.trim().to_owned(),
            location: self.location.trim().to_owned(),
            description: self.description.trim().to_owned(),
            reporter: self.reporter.map(|reporter| Reporter {
                name: reporter.name.trim().to_owned(),
                email: reporter
                    .email
                    .map(|email| email.trim().to_owned())
                    .filter(|email| !email.is_empty()),
            }),
            ..self
        }
    }

    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();
        check_length(
            &mut errors,
            Field::Title,
            &self.title,
            Self::TITLE_LEN,
            "El título debe tener entre 4 y 120 caracteres.",
        );
        check_length(
            &mut errors,
            Field::Location,
            &self.location,
            Self::LOCATION_LEN,
            "Indica la ubicación (entre 2 y 120 caracteres).",
        );
        check_length(
            &mut errors,
            Field::Description,
            &self.description,
            Self::DESCRIPTION_LEN,
            "Describe la situación con al menos 10 caracteres (máximo 2000).",
        );
        if let Some(reporter) = &self.reporter {
            check_length(
                &mut errors,
                Field::ReporterName,
                &reporter.name,
                Self::NAME_LEN,
                "Escribe tu nombre o envía el reporte de forma anónima.",
            );
            if let Some(email) = &reporter.email {
                if !is_plausible_email(email) {
                    errors.push(Field::ReporterEmail, "El correo no parece válido.");
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

fn check_length(
    errors: &mut ValidationErrors,
    field: Field,
    value: &str,
    range: RangeInclusive<usize>,
    message: &'static str,
) {
    if !range.contains(&value.trim().chars().count()) {
        errors.push(field, message);
    }
}

/// Validación sintáctica sencilla: `algo@dominio.ext`, sin espacios.
pub fn is_plausible_email(email: &str) -> bool {
    let email = email.trim();
    if email.chars().any(char::is_whitespace) {
        return false;
    }
    match email.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && !domain.contains('@')
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        }
        None => false,
    }
}

/// Reporte persistido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub id: ReportId,
    pub category: Category,
    pub severity: Severity,
    pub status: Status,
    pub title: String,
    pub location: String,
    pub description: String,
    pub reporter: Option<Reporter>,
    /// Segundos Unix.
    pub created_at: i64,
    /// `true` cuando la notificación por correo ya fue entregada.
    pub notified: bool,
}

impl Report {
    pub fn is_active(&self) -> bool {
        self.status != Status::Resolved
    }

    pub fn reporter_display(&self) -> String {
        match &self.reporter {
            None => "Anónimo".to_owned(),
            Some(Reporter { name, email: Some(email) }) => format!("{name} <{email}>"),
            Some(Reporter { name, email: None }) => name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> NewReport {
        NewReport {
            category: Category::PoorLighting,
            severity: Severity::High,
            title: "Parqueadero sin luz".into(),
            location: "Parqueadero norte".into(),
            description: "Las lámparas llevan una semana apagadas.".into(),
            reporter: None,
        }
    }

    #[test]
    fn valid_anonymous_report_passes() {
        assert!(draft().validate().is_ok());
    }

    #[test]
    fn short_fields_are_reported_per_field() {
        let report = NewReport { title: "ab".into(), description: "corta".into(), ..draft() };
        let errors = report.validate().unwrap_err();
        assert!(errors.get(Field::Title).is_some());
        assert!(errors.get(Field::Description).is_some());
        assert!(errors.get(Field::Location).is_none());
    }

    #[test]
    fn identified_reporter_requires_name_and_valid_email() {
        let report = NewReport {
            reporter: Some(Reporter { name: " ".into(), email: Some("sin-arroba".into()) }),
            ..draft()
        };
        let errors = report.validate().unwrap_err();
        assert!(errors.get(Field::ReporterName).is_some());
        assert!(errors.get(Field::ReporterEmail).is_some());
    }

    #[test]
    fn normalization_trims_and_drops_empty_email() {
        let report = NewReport {
            title: "  Baranda suelta  ".into(),
            reporter: Some(Reporter { name: " Ana ".into(), email: Some("   ".into()) }),
            ..draft()
        }
        .normalized();
        assert_eq!(report.title, "Baranda suelta");
        assert_eq!(report.reporter, Some(Reporter { name: "Ana".into(), email: None }));
    }

    #[test]
    fn email_heuristics() {
        assert!(is_plausible_email("ana@umb.edu.co"));
        assert!(!is_plausible_email("ana@umb"));
        assert!(!is_plausible_email("ana @umb.co"));
        assert!(!is_plausible_email("@umb.co"));
    }

    #[test]
    fn codes_round_trip() {
        for &c in Category::ALL {
            assert_eq!(Category::from_code(c.code()), Some(c));
        }
        for &s in Severity::ALL {
            assert_eq!(Severity::from_code(s.code()), Some(s));
        }
        for &s in Status::ALL {
            assert_eq!(Status::from_code(s.code()), Some(s));
        }
        assert_eq!(Status::from_code("desconocido"), None);
    }
}
