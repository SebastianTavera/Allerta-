//! Faro — plataforma de detección de situaciones de riesgo en universidades.
//!
//! `main` es la *raíz de composición*: el único punto que conoce las
//! implementaciones concretas (SQLite, SMTP, reloj del sistema) y las conecta
//! con las abstracciones del dominio (principio de inversión de dependencias).

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod application;
mod domain;
mod infrastructure;
mod ui;

use std::sync::Arc;

use application::{NotificationDispatcher, ReportService};
use domain::{NotificationOutbox, Notifier, ReportRepository};
use infrastructure::{AppConfig, DisabledNotifier, SmtpNotifier, SqliteReportRepository, SystemClock};

fn main() -> eframe::Result<()> {
    let config = AppConfig::load();

    let repository = match SqliteReportRepository::open(&config.database_path) {
        Ok(repository) => Arc::new(repository),
        Err(err) => {
            eprintln!(
                "Faro: no se pudo abrir la base de datos {}: {err}",
                config.database_path.display()
            );
            std::process::exit(1);
        }
    };
    let notifier = build_notifier(&config);

    eframe::run_native(
        "Faro",
        ui::native_options(),
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let outbox: Arc<dyn NotificationOutbox> = repository.clone();
            let reports: Arc<dyn ReportRepository> = repository;
            let dispatcher = NotificationDispatcher::spawn(
                notifier.clone(),
                outbox.clone(),
                Arc::new(move || ctx.request_repaint()),
            );
            let service = ReportService::new(
                reports,
                outbox,
                Arc::new(SystemClock),
                dispatcher,
                notifier.status(),
            );
            Ok(Box::new(ui::FaroApp::new(cc, service)))
        }),
    )
}

/// Elige el canal de notificación según la configuración disponible.
fn build_notifier(config: &AppConfig) -> Arc<dyn Notifier> {
    match &config.smtp {
        Some(smtp) => match SmtpNotifier::new(smtp, &config.recipient) {
            Ok(notifier) => Arc::new(notifier),
            Err(err) => Arc::new(DisabledNotifier::new(&config.recipient, err.to_string())),
        },
        None => Arc::new(DisabledNotifier::new(
            &config.recipient,
            "Configura SMTP en faro.env para enviar correos",
        )),
    }
}
