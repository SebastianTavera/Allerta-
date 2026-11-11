//! Capa de aplicación: casos de uso que orquestan el dominio.
//! Depende solo de abstracciones del dominio, nunca de SQLite, SMTP o la UI.

mod dispatcher;
pub mod format;
mod notification_message;
mod report_service;
mod summary;

pub use dispatcher::{NotificationDispatcher, NotificationEvent};
pub use notification_message::NotificationMessage;
pub use report_service::{ReportService, ServiceError};
pub use summary::Summary;
