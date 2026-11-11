//! Caso de uso principal: registrar, consultar y gestionar reportes.

use std::fmt;
use std::sync::Arc;

use super::dispatcher::{NotificationDispatcher, NotificationEvent};
use crate::domain::{
    Clock, NewReport, NotificationOutbox, NotifierStatus, Report, ReportId, ReportRepository,
    RepositoryError, Status, ValidationErrors,
};

#[derive(Debug)]
pub enum ServiceError {
    Invalid(ValidationErrors),
    Storage(RepositoryError),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(errors) => write!(f, "{errors}"),
            Self::Storage(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl From<ValidationErrors> for ServiceError {
    fn from(errors: ValidationErrors) -> Self {
        Self::Invalid(errors)
    }
}

impl From<RepositoryError> for ServiceError {
    fn from(err: RepositoryError) -> Self {
        Self::Storage(err)
    }
}

pub struct ReportService {
    reports: Arc<dyn ReportRepository>,
    outbox: Arc<dyn NotificationOutbox>,
    clock: Arc<dyn Clock>,
    dispatcher: NotificationDispatcher,
    notifier: NotifierStatus,
}

impl ReportService {
    pub fn new(
        reports: Arc<dyn ReportRepository>,
        outbox: Arc<dyn NotificationOutbox>,
        clock: Arc<dyn Clock>,
        dispatcher: NotificationDispatcher,
        notifier: NotifierStatus,
    ) -> Self {
        Self { reports, outbox, clock, dispatcher, notifier }
    }

    /// Valida, guarda y encola la notificación por correo. Devuelve de
    /// inmediato: el envío ocurre en segundo plano.
    pub fn submit(&self, draft: NewReport) -> Result<Report, ServiceError> {
        let draft = draft.normalized();
        draft.validate()?;
        let report = self.reports.insert(&draft, self.clock.now())?;
        self.dispatcher.enqueue(report.clone());
        Ok(report)
    }

    pub fn reports(&self) -> Result<Vec<Report>, ServiceError> {
        Ok(self.reports.all()?)
    }

    pub fn change_status(&self, id: ReportId, status: Status) -> Result<(), ServiceError> {
        Ok(self.reports.set_status(id, status)?)
    }

    /// Vuelve a encolar los reportes cuya notificación no se entregó.
    pub fn retry_pending(&self) -> Result<usize, ServiceError> {
        let pending = self.outbox.pending()?;
        let count = pending.len();
        pending.into_iter().for_each(|report| self.dispatcher.enqueue(report));
        Ok(count)
    }

    pub fn poll_notifications(&self) -> Vec<NotificationEvent> {
        self.dispatcher.drain()
    }

    pub fn notifier(&self) -> &NotifierStatus {
        &self.notifier
    }

    pub fn now(&self) -> i64 {
        self.clock.now()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::domain::{Category, Notifier, NotifyError, Severity};
    use crate::infrastructure::SqliteReportRepository;

    struct FixedClock;
    impl Clock for FixedClock {
        fn now(&self) -> i64 {
            1_700_000_000
        }
    }

    #[derive(Default)]
    struct RecordingNotifier {
        sent: Mutex<Vec<ReportId>>,
        fail: bool,
    }

    impl Notifier for RecordingNotifier {
        fn notify(&self, report: &Report) -> Result<(), NotifyError> {
            if self.fail {
                return Err(NotifyError::Delivery("sin conexión".into()));
            }
            self.sent.lock().unwrap().push(report.id);
            Ok(())
        }

        fn status(&self) -> NotifierStatus {
            NotifierStatus { enabled: true, destination: "test@umb.edu.co".into(), detail: String::new() }
        }
    }

    fn service(notifier: Arc<RecordingNotifier>) -> ReportService {
        let repository = Arc::new(SqliteReportRepository::in_memory().unwrap());
        let outbox: Arc<dyn NotificationOutbox> = repository.clone();
        let dispatcher = NotificationDispatcher::spawn(notifier.clone(), outbox.clone(), Arc::new(|| {}));
        ReportService::new(repository, outbox, Arc::new(FixedClock), dispatcher, notifier.status())
    }

    fn draft() -> NewReport {
        NewReport {
            category: Category::Emergency,
            severity: Severity::Critical,
            title: "Humo en laboratorio".into(),
            location: "Laboratorio 3".into(),
            description: "Sale humo de un tomacorriente.".into(),
            reporter: None,
        }
    }

    fn wait_events(service: &ReportService) -> Vec<NotificationEvent> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let events = service.poll_notifications();
            if !events.is_empty() || Instant::now() > deadline {
                return events;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn invalid_report_is_not_stored() {
        let service = service(Arc::default());
        let result = service.submit(NewReport { title: "x".into(), ..draft() });
        assert!(matches!(result, Err(ServiceError::Invalid(_))));
        assert!(service.reports().unwrap().is_empty());
    }

    #[test]
    fn valid_report_is_stored_and_notified() {
        let notifier = Arc::new(RecordingNotifier::default());
        let service = service(notifier.clone());
        let report = service.submit(draft()).unwrap();

        assert_eq!(wait_events(&service), vec![NotificationEvent::Delivered(report.id)]);
        assert_eq!(*notifier.sent.lock().unwrap(), vec![report.id]);
        assert!(service.reports().unwrap()[0].notified);
    }

    #[test]
    fn failed_notification_stays_pending() {
        let notifier = Arc::new(RecordingNotifier { fail: true, ..Default::default() });
        let service = service(notifier);
        let report = service.submit(draft()).unwrap();

        assert!(matches!(wait_events(&service)[0], NotificationEvent::Failed { id, .. } if id == report.id));
        assert!(!service.reports().unwrap()[0].notified);
        assert_eq!(service.retry_pending().unwrap(), 1);
    }
}
