//! Capa de dominio: entidades, reglas de negocio y contratos (traits).
//!
//! No depende de ninguna otra capa ni de bibliotecas externas: la base de
//! datos, el correo y la interfaz se adaptan a estos contratos, no al revés.

pub mod clock;
pub mod notifier;
pub mod report;
pub mod repository;

pub use clock::Clock;
pub use notifier::{Notifier, NotifierStatus, NotifyError};
pub use report::{
    Category, Coded, Field, NewReport, Report, ReportId, Reporter, Severity, Status,
    ValidationErrors,
};
pub use repository::{NotificationOutbox, ReportRepository, RepositoryError};
