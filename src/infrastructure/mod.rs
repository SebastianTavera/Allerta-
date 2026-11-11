//! Capa de infraestructura: adaptadores concretos de los contratos del dominio.

mod config;
mod disabled_notifier;
mod smtp_notifier;
mod sqlite_repository;
mod system_clock;

pub use config::AppConfig;
pub use disabled_notifier::DisabledNotifier;
pub use smtp_notifier::SmtpNotifier;
pub use sqlite_repository::SqliteReportRepository;
pub use system_clock::SystemClock;
