//! Contratos de persistencia. Se separan en dos interfaces pequeñas
//! (principio de segregación de interfaces): quien solo despacha
//! notificaciones no necesita conocer el resto del repositorio.

use std::fmt;

use super::report::{NewReport, Report, ReportId, Status};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryError(String);

impl RepositoryError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RepositoryError {}

/// Almacenamiento de reportes.
pub trait ReportRepository: Send + Sync {
    fn insert(&self, report: &NewReport, created_at: i64) -> Result<Report, RepositoryError>;
    /// Todos los reportes, del más reciente al más antiguo.
    fn all(&self) -> Result<Vec<Report>, RepositoryError>;
    fn set_status(&self, id: ReportId, status: Status) -> Result<(), RepositoryError>;
}

/// Bandeja de salida de notificaciones (patrón *outbox*): garantiza que un
/// reporte nunca se pierda aunque el correo falle; se reintenta después.
pub trait NotificationOutbox: Send + Sync {
    fn pending(&self) -> Result<Vec<Report>, RepositoryError>;
    fn mark_notified(&self, id: ReportId) -> Result<(), RepositoryError>;
}
