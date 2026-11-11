use std::fmt;

use super::report::Report;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyError {
    /// El canal no está configurado o fue deshabilitado.
    Disabled(String),
    /// El canal está activo pero la entrega falló.
    Delivery(String),
}

impl fmt::Display for NotifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled(reason) => f.write_str(reason),
            Self::Delivery(reason) => write!(f, "falló el envío: {reason}"),
        }
    }
}

impl std::error::Error for NotifyError {}

/// Estado legible de un canal de notificación, para mostrarlo en la interfaz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifierStatus {
    pub enabled: bool,
    pub destination: String,
    pub detail: String,
}

/// Canal que avisa a los responsables cuando se crea un reporte.
/// Nuevos canales (Telegram, SMS, webhooks…) se agregan implementando este
/// trait, sin modificar el resto del sistema (principio abierto/cerrado).
pub trait Notifier: Send + Sync {
    fn notify(&self, report: &Report) -> Result<(), NotifyError>;
    fn status(&self) -> NotifierStatus;
}
