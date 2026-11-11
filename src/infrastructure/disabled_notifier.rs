use crate::domain::{Notifier, NotifierStatus, NotifyError, Report};

/// Objeto nulo: se usa cuando el correo no está configurado. Los reportes se
/// siguen guardando y quedan pendientes en la bandeja de salida.
pub struct DisabledNotifier {
    destination: String,
    reason: String,
}

impl DisabledNotifier {
    pub fn new(destination: &str, reason: impl Into<String>) -> Self {
        Self { destination: destination.to_owned(), reason: reason.into() }
    }
}

impl Notifier for DisabledNotifier {
    fn notify(&self, _report: &Report) -> Result<(), NotifyError> {
        Err(NotifyError::Disabled(self.reason.clone()))
    }

    fn status(&self) -> NotifierStatus {
        NotifierStatus {
            enabled: false,
            destination: self.destination.clone(),
            detail: self.reason.clone(),
        }
    }
}
