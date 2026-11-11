//! Repositorio SQLite. Una única conexión protegida por `Mutex` basta para
//! una aplicación de escritorio y mantiene el consumo de memoria mínimo.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, Row};

use crate::domain::{
    Category, Coded, NewReport, NotificationOutbox, Report, ReportId, ReportRepository, Reporter,
    RepositoryError, Severity, Status,
};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS reports (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    category       TEXT    NOT NULL,
    severity       TEXT    NOT NULL,
    status         TEXT    NOT NULL DEFAULT 'open',
    title          TEXT    NOT NULL,
    location       TEXT    NOT NULL,
    description    TEXT    NOT NULL,
    reporter_name  TEXT,
    reporter_email TEXT,
    created_at     INTEGER NOT NULL,
    notified       INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_reports_created_at ON reports (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_reports_pending ON reports (notified) WHERE notified = 0;
";

const COLUMNS: &str = "id, category, severity, status, title, location, description, \
                       reporter_name, reporter_email, created_at, notified";

pub struct SqliteReportRepository {
    connection: Mutex<Connection>,
}

impl SqliteReportRepository {
    pub fn open(path: &Path) -> Result<Self, RepositoryError> {
        if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|err| {
                RepositoryError::new(format!("no se pudo crear {}: {err}", dir.display()))
            })?;
        }
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")
            .map_err(storage)?;
        Self::initialize(connection)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn in_memory() -> Result<Self, RepositoryError> {
        Self::initialize(Connection::open_in_memory().map_err(storage)?)
    }

    fn initialize(connection: Connection) -> Result<Self, RepositoryError> {
        connection.execute_batch(SCHEMA).map_err(storage)?;
        Ok(Self { connection: Mutex::new(connection) })
    }

    fn run<T>(&self, operation: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T, RepositoryError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| RepositoryError::new("la conexión a la base de datos quedó bloqueada"))?;
        operation(&connection).map_err(storage)
    }

    fn select(&self, filter: &str, order: &str) -> Result<Vec<Report>, RepositoryError> {
        let sql = format!("SELECT {COLUMNS} FROM reports {filter} ORDER BY {order}");
        self.run(|connection| {
            let mut statement = connection.prepare_cached(&sql)?;
            let reports = statement.query_map([], map_row)?.collect::<rusqlite::Result<Vec<_>>>();
            reports
        })
    }
}

fn storage(err: rusqlite::Error) -> RepositoryError {
    RepositoryError::new(format!("error de base de datos: {err}"))
}

fn map_row(row: &Row<'_>) -> rusqlite::Result<Report> {
    let category: String = row.get(1)?;
    let severity: String = row.get(2)?;
    let status: String = row.get(3)?;
    let reporter_name: Option<String> = row.get(7)?;
    let reporter_email: Option<String> = row.get(8)?;

    Ok(Report {
        id: ReportId(row.get(0)?),
        category: Category::from_code(&category).unwrap_or(Category::OtherRisk),
        severity: Severity::from_code(&severity).unwrap_or(Severity::Medium),
        status: Status::from_code(&status).unwrap_or(Status::Open),
        title: row.get(4)?,
        location: row.get(5)?,
        description: row.get(6)?,
        reporter: reporter_name.map(|name| Reporter { name, email: reporter_email }),
        created_at: row.get(9)?,
        notified: row.get::<_, i64>(10)? != 0,
    })
}

impl ReportRepository for SqliteReportRepository {
    fn insert(&self, report: &NewReport, created_at: i64) -> Result<Report, RepositoryError> {
        let (name, email) = match &report.reporter {
            Some(reporter) => (Some(reporter.name.as_str()), reporter.email.as_deref()),
            None => (None, None),
        };
        let id = self.run(|connection| {
            connection.execute(
                "INSERT INTO reports (category, severity, status, title, location, description, \
                 reporter_name, reporter_email, created_at, notified) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0)",
                params![
                    report.category.code(),
                    report.severity.code(),
                    Status::Open.code(),
                    report.title,
                    report.location,
                    report.description,
                    name,
                    email,
                    created_at,
                ],
            )?;
            Ok(connection.last_insert_rowid())
        })?;

        Ok(Report {
            id: ReportId(id),
            category: report.category,
            severity: report.severity,
            status: Status::Open,
            title: report.title.clone(),
            location: report.location.clone(),
            description: report.description.clone(),
            reporter: report.reporter.clone(),
            created_at,
            notified: false,
        })
    }

    fn all(&self) -> Result<Vec<Report>, RepositoryError> {
        self.select("", "created_at DESC, id DESC")
    }

    fn set_status(&self, id: ReportId, status: Status) -> Result<(), RepositoryError> {
        let changed = self.run(|connection| {
            connection.execute("UPDATE reports SET status = ?1 WHERE id = ?2", params![status.code(), id.0])
        })?;
        if changed == 0 {
            return Err(RepositoryError::new(format!("el reporte {id} no existe")));
        }
        Ok(())
    }
}

impl NotificationOutbox for SqliteReportRepository {
    fn pending(&self) -> Result<Vec<Report>, RepositoryError> {
        self.select("WHERE notified = 0", "created_at ASC, id ASC")
    }

    fn mark_notified(&self, id: ReportId) -> Result<(), RepositoryError> {
        self.run(|connection| connection.execute("UPDATE reports SET notified = 1 WHERE id = ?1", params![id.0]))
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(title: &str, reporter: Option<Reporter>) -> NewReport {
        NewReport {
            category: Category::Harassment,
            severity: Severity::High,
            title: title.into(),
            location: "Biblioteca".into(),
            description: "Descripción de prueba.".into(),
            reporter,
        }
    }

    #[test]
    fn insert_and_list_newest_first() {
        let repo = SqliteReportRepository::in_memory().unwrap();
        let reporter = Some(Reporter { name: "Ana".into(), email: Some("ana@umb.edu.co".into()) });
        repo.insert(&draft("Primero", None), 10).unwrap();
        let second = repo.insert(&draft("Segundo", reporter.clone()), 20).unwrap();

        let all = repo.all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], second);
        assert_eq!(all[0].reporter, reporter);
        assert_eq!(all[1].reporter, None);
    }

    #[test]
    fn status_updates_and_missing_ids_fail() {
        let repo = SqliteReportRepository::in_memory().unwrap();
        let report = repo.insert(&draft("Estado", None), 10).unwrap();
        repo.set_status(report.id, Status::Resolved).unwrap();
        assert_eq!(repo.all().unwrap()[0].status, Status::Resolved);
        assert!(repo.set_status(ReportId(999), Status::Open).is_err());
    }

    #[test]
    fn outbox_tracks_pending_notifications() {
        let repo = SqliteReportRepository::in_memory().unwrap();
        let a = repo.insert(&draft("A", None), 10).unwrap();
        let b = repo.insert(&draft("B", None), 20).unwrap();
        repo.mark_notified(a.id).unwrap();

        let pending = repo.pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, b.id);
    }
}
