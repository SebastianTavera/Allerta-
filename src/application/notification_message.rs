//! Contenido de la notificación, independiente del canal que la entregue.

use super::format;
use crate::domain::{Coded, Report, Severity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationMessage {
    pub subject: String,
    pub body: String,
}

impl NotificationMessage {
    pub fn for_report(report: &Report) -> Self {
        let urgent = report.category.is_urgent() || report.severity == Severity::Critical;
        let prefix = if urgent { "[URGENTE] " } else { "" };
        let subject = format!(
            "{prefix}Faro · {} · {} — {}",
            report.category.label(),
            report.severity.label(),
            report.title
        );

        let underline = "─".repeat(report.title.chars().count().clamp(3, 60));
        let body = format!(
            "Se registró un nuevo reporte en Faro.\n\
             \n\
             Reporte:        {id}\n\
             Categoría:      {category}\n\
             Gravedad:       {severity}\n\
             Ubicación:      {location}\n\
             Fecha:          {date}\n\
             Reportado por:  {reporter}\n\
             \n\
             {title}\n\
             {underline}\n\
             {description}\n\
             \n\
             —\n\
             Mensaje automático de Faro · Plataforma de seguridad del campus.\n",
            id = report.id,
            category = report.category.label(),
            severity = report.severity.label(),
            location = report.location,
            date = format::datetime(report.created_at),
            reporter = report.reporter_display(),
            title = report.title,
            description = report.description,
        );

        Self { subject, body }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Category, ReportId, Status};

    fn report(category: Category, severity: Severity) -> Report {
        Report {
            id: ReportId(7),
            category,
            severity,
            status: Status::Open,
            title: "Escalera dañada".into(),
            location: "Bloque B".into(),
            description: "El tercer escalón está suelto.".into(),
            reporter: None,
            created_at: 0,
            notified: false,
        }
    }

    #[test]
    fn message_contains_key_data() {
        let message =
            NotificationMessage::for_report(&report(Category::FacilityDamage, Severity::Medium));
        assert!(message.subject.contains("Daño en instalaciones"));
        assert!(!message.subject.starts_with("[URGENTE]"));
        assert!(message.body.contains("#0007"));
        assert!(message.body.contains("Bloque B"));
        assert!(message.body.contains("Anónimo"));
    }

    #[test]
    fn urgent_categories_are_flagged() {
        let message = NotificationMessage::for_report(&report(Category::Harassment, Severity::Low));
        assert!(message.subject.starts_with("[URGENTE]"));
    }
}
