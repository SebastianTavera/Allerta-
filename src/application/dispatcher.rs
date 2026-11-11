//! Despacho de notificaciones en segundo plano: la interfaz nunca se bloquea
//! esperando al servidor de correo.

use std::collections::HashSet;
use std::sync::{mpsc, Arc, Mutex, PoisonError};
use std::thread;

use crate::domain::{NotificationOutbox, Notifier, Report, ReportId};

/// Función que despierta a la interfaz cuando hay eventos nuevos.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationEvent {
    Delivered(ReportId),
    Failed { id: ReportId, reason: String },
}

pub struct NotificationDispatcher {
    jobs: mpsc::Sender<Report>,
    events: mpsc::Receiver<NotificationEvent>,
    in_flight: Arc<Mutex<HashSet<ReportId>>>,
}

impl NotificationDispatcher {
    pub fn spawn(
        notifier: Arc<dyn Notifier>,
        outbox: Arc<dyn NotificationOutbox>,
        waker: Waker,
    ) -> Self {
        let (jobs, queue) = mpsc::channel::<Report>();
        let (event_sender, events) = mpsc::channel();
        let in_flight = Arc::new(Mutex::new(HashSet::new()));
        let worker_in_flight = Arc::clone(&in_flight);

        thread::Builder::new()
            .name("faro-notificaciones".into())
            .spawn(move || {
                for report in queue {
                    let event = deliver(notifier.as_ref(), outbox.as_ref(), &report);
                    worker_in_flight
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .remove(&report.id);
                    if event_sender.send(event).is_err() {
                        break;
                    }
                    waker();
                }
            })
            .expect("no se pudo iniciar el hilo de notificaciones");

        Self { jobs, events, in_flight }
    }

    /// Encola un reporte. Si ya está en cola, no se duplica el correo.
    pub fn enqueue(&self, report: Report) {
        let newly_queued = self
            .in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(report.id);
        if newly_queued {
            // Si el hilo terminó, el reporte queda pendiente en la bandeja de
            // salida y se reintentará en el próximo inicio.
            let _ = self.jobs.send(report);
        }
    }

    /// Eventos ocurridos desde la última consulta (no bloquea).
    pub fn drain(&self) -> Vec<NotificationEvent> {
        self.events.try_iter().collect()
    }
}

fn deliver(
    notifier: &dyn Notifier,
    outbox: &dyn NotificationOutbox,
    report: &Report,
) -> NotificationEvent {
    let result = notifier
        .notify(report)
        .map_err(|err| err.to_string())
        .and_then(|()| outbox.mark_notified(report.id).map_err(|err| err.to_string()));
    match result {
        Ok(()) => NotificationEvent::Delivered(report.id),
        Err(reason) => NotificationEvent::Failed { id: report.id, reason },
    }
}
