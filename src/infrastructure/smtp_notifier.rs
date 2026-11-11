use std::time::Duration;

use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use super::config::SmtpConfig;
use crate::application::NotificationMessage;
use crate::domain::{Notifier, NotifierStatus, NotifyError, Report};

const SMTPS_PORT: u16 = 465;
const TIMEOUT: Duration = Duration::from_secs(20);

/// Notificador por correo electrónico vía SMTP (TLS con rustls).
pub struct SmtpNotifier {
    transport: SmtpTransport,
    from: Mailbox,
    to: Mailbox,
    host: String,
}

impl SmtpNotifier {
    pub fn new(config: &SmtpConfig, recipient: &str) -> Result<Self, NotifyError> {
        let from: Mailbox = config
            .from
            .parse()
            .map_err(|err| NotifyError::Disabled(format!("Remitente inválido ({}): {err}", config.from)))?;
        let to: Mailbox = recipient
            .parse()
            .map_err(|err| NotifyError::Disabled(format!("Destinatario inválido ({recipient}): {err}")))?;

        // 465 usa TLS implícito; cualquier otro puerto (587) usa STARTTLS.
        let builder = if config.port == SMTPS_PORT {
            SmtpTransport::relay(&config.host)
        } else {
            SmtpTransport::starttls_relay(&config.host)
        }
        .map_err(|err| NotifyError::Disabled(format!("Servidor SMTP inválido: {err}")))?;

        let transport = builder
            .port(config.port)
            .credentials(Credentials::new(config.username.clone(), config.password.clone()))
            .timeout(Some(TIMEOUT))
            .build();

        Ok(Self { transport, from, to, host: config.host.clone() })
    }
}

impl Notifier for SmtpNotifier {
    fn notify(&self, report: &Report) -> Result<(), NotifyError> {
        let message = NotificationMessage::for_report(report);
        let mut builder = Message::builder()
            .from(self.from.clone())
            .to(self.to.clone())
            .subject(message.subject);

        // Si la persona dejó su correo, responder llega directamente a ella.
        let reply_to = report
            .reporter
            .as_ref()
            .and_then(|reporter| reporter.email.as_deref())
            .and_then(|email| email.parse::<Mailbox>().ok());
        if let Some(reply_to) = reply_to {
            builder = builder.reply_to(reply_to);
        }

        let email = builder
            .header(ContentType::TEXT_PLAIN)
            .body(message.body)
            .map_err(|err| NotifyError::Delivery(err.to_string()))?;

        self.transport
            .send(&email)
            .map(|_| ())
            .map_err(|err| NotifyError::Delivery(err.to_string()))
    }

    fn status(&self) -> NotifierStatus {
        NotifierStatus {
            enabled: true,
            destination: self.to.email.to_string(),
            detail: format!("vía {}", self.host),
        }
    }
}
