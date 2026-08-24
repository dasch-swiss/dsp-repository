//! Sending the one-time code (REQ-6.7), and what happens when there is no relay
//! (REQ-6.8) or the relay refuses (REQ-6.9).
//!
//! ## Why no error message ever reaches a log
//!
//! REQ-6.10 forbids an account holder's address in a log or a trace, and an SMTP
//! failure is the one place such an address arrives from outside our own code: a
//! relay's reply text routinely echoes the recipient — `550 5.1.1
//! <someone@example.org> User unknown` is the canonical shape. Logging the
//! transport error verbatim would therefore write addresses into Grafana on
//! exactly the paths nobody tests.
//!
//! The configured sender is a different matter and is logged on purpose: see
//! [`SmtpMailer::describe`].
//!
//! So [`MailError`] carries a **classification and the three-digit status code**,
//! and never the reply text. What is lost is the relay's prose; what is kept is
//! everything an operator acts on — permanent versus transient, and the code.

use async_trait::async_trait;

/// One message, ready to hand to a transport.
pub(crate) struct Mail {
    /// The recipient. Never logged, never rendered.
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Why a message was not delivered.
///
/// Deliberately coarse. See the module docs: the relay's own words are the leak.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum MailError {
    /// The relay answered and refused. `permanent` separates "this address will
    /// never work" from "try again later", which is the distinction that decides
    /// whether an operator investigates now.
    #[error("the relay refused the message (permanent: {permanent}, status: {})", .status.as_deref().unwrap_or("none"))]
    Refused { permanent: bool, status: Option<String> },

    /// The relay could not be reached, or the exchange failed before a reply —
    /// DNS, connection, TLS, timeout.
    #[error("the relay could not be reached")]
    Unreachable,

    /// The message could not be built: a stored address that is not a valid
    /// mailbox, which is a data problem rather than a transport one.
    #[error("the message could not be built")]
    Malformed,
}

/// A transport for [`Mail`].
#[async_trait]
pub(crate) trait Mailer: Send + Sync {
    async fn send(&self, mail: &Mail) -> Result<(), MailError>;

    /// How this transport is reported at startup, so an operator can see from
    /// the first log line whether mail actually leaves the process.
    fn describe(&self) -> String;
}

/// REQ-6.8: no relay configured, so the message goes to the log and the service
/// stays usable.
///
/// This is the development and PR-preview transport, and it is the break-glass
/// for a broken relay: unsetting `EDITOR_SMTP_HOST` routes every code here.
pub(crate) struct ConsoleMailer;

#[async_trait]
impl Mailer for ConsoleMailer {
    async fn send(&self, mail: &Mail) -> Result<(), MailError> {
        // The recipient is deliberately absent (REQ-6.10). Whoever is reading
        // this log typed the address themselves a moment ago; the enclosing
        // span carries the opaque subject id for the case where two people are
        // testing at once.
        tracing::warn!(
            mail.subject = %mail.subject,
            mail.body = %mail.body,
            "no SMTP relay is configured — writing the message to the log instead of sending it"
        );
        Ok(())
    }

    fn describe(&self) -> String {
        "console — EDITOR_SMTP_HOST is unset, so login codes are written to the log".to_string()
    }
}

/// REQ-6.7: the Google Workspace relay, over STARTTLS.
pub(crate) struct SmtpMailer {
    transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
    from: lettre::message::Mailbox,
    host: String,
    port: u16,
}

/// A relay that could not be configured at all, which stops startup rather than
/// failing at the first login.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SmtpSetupError {
    #[error("EDITOR_SMTP_FROM is not a valid mailbox: {0}")]
    From(String),
    #[error("the SMTP relay could not be configured: {0}")]
    Relay(String),
}

impl SmtpMailer {
    /// Build the transport. Does not connect — a relay that is down must not
    /// stop the process, only fail the sends it is asked for.
    pub(crate) fn new(
        host: &str,
        port: u16,
        credentials: Option<(&str, &str)>,
        from: &str,
    ) -> Result<Self, SmtpSetupError> {
        use lettre::transport::smtp::authentication::Credentials;

        let from: lettre::message::Mailbox = from.parse().map_err(|e| SmtpSetupError::From(format!("{e}")))?;

        // STARTTLS (RFC 2487) on 587, which is what `smtp-relay.gmail.com`
        // speaks. `relay()` would be implicit TLS on 465; `starttls_relay` still
        // requires the upgrade to succeed, so this is not an opportunistic
        // downgrade path.
        // `format!("{e}")` on an SMTP error is the operation this module's docs
        // forbid: `Display` on `lettre::transport::smtp::Error` appends its
        // source, and for a *connected* transport that source is the relay's
        // reply — which quotes the recipient.
        //
        // It is safe here only because of where it sits. `starttls_relay` does
        // exactly one fallible thing, `TlsParameters::new(host)`, and at that
        // point there is no message, no envelope, no connection and no reply;
        // the only external content reachable is `EDITOR_SMTP_HOST`, which is
        // ours. Credentials are attached below, after this call, so no password
        // can be in it either.
        //
        // If anything ever routes a connected transport's error through
        // `SmtpSetupError::Relay` — a startup connectivity probe, a
        // `test_connection()`, a retry wrapper — that reasoning stops holding and
        // this must go through `classify` like the send path does.
        let mut builder = lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::starttls_relay(host)
            .map_err(|e| SmtpSetupError::Relay(format!("{e}")))?
            .port(port);
        if let Some((username, password)) = credentials {
            builder = builder.credentials(Credentials::new(username.to_string(), password.to_string()));
        }

        Ok(Self {
            transport: builder.build(),
            from,
            host: host.to_string(),
            port,
        })
    }
}

/// Reduce a transport error to what may be logged.
///
/// See the module docs: `error.to_string()` on an SMTP error can contain the
/// relay's reply, and a relay's reply routinely contains the recipient.
fn classify(error: &lettre::transport::smtp::Error) -> MailError {
    if error.is_response() {
        MailError::Refused {
            permanent: error.is_permanent(),
            status: error.status().map(|code| format!("{code}")),
        }
    } else {
        MailError::Unreachable
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, mail: &Mail) -> Result<(), MailError> {
        use lettre::message::header::ContentType;
        use lettre::{AsyncTransport, Message};

        let to: lettre::message::Mailbox = mail.to.parse().map_err(|_| MailError::Malformed)?;
        let message = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(&mail.subject)
            .header(ContentType::TEXT_PLAIN)
            .body(mail.body.clone())
            .map_err(|_| MailError::Malformed)?;

        self.transport.send(message).await.map_err(|e| classify(&e))?;
        Ok(())
    }

    fn describe(&self) -> String {
        format!("SMTP {}:{} (STARTTLS) as {}", self.host, self.port, self.from.email)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a_refusal_reports_the_status_and_nothing_the_relay_said() {
        // The property the module exists for: an SMTP reply is
        // `550 5.1.1 <someone@example.org> User unknown`, and the address in it
        // must not survive into a log line.
        let error = MailError::Refused { permanent: true, status: Some("550".to_string()) };
        let rendered = error.to_string();
        assert!(rendered.contains("550"), "{rendered}");
        assert!(rendered.contains("permanent: true"), "{rendered}");
        assert!(!rendered.contains('@'), "no address may appear in a mail error: {rendered}");
    }

    #[test]
    fn test_every_mail_error_is_free_of_an_address() {
        for error in [
            MailError::Refused { permanent: false, status: Some("451".to_string()) },
            MailError::Refused { permanent: true, status: None },
            MailError::Unreachable,
            MailError::Malformed,
        ] {
            assert!(!error.to_string().contains('@'), "{error}");
        }
    }

    #[test]
    fn test_an_invalid_from_address_stops_startup() {
        // Reported at startup rather than at the first login: an unparseable
        // `EDITOR_SMTP_FROM` makes every send fail, and finding that out from a
        // user report is the expensive way.
        // Matched rather than unwrapped: the transport itself is not `Debug`,
        // deliberately — it holds credentials.
        let result = SmtpMailer::new("smtp.example.test", 587, None, "not a mailbox");
        assert!(
            matches!(result.as_ref().err(), Some(SmtpSetupError::From(_))),
            "an invalid From must be refused, got ok: {}",
            result.is_ok()
        );
    }

    // Async because `AsyncSmtpTransport` holds a connection pool whose drop
    // needs a reactor; building one outside a runtime aborts the process on
    // cleanup rather than failing the test.
    #[tokio::test]
    async fn test_a_configured_relay_describes_itself_without_credentials() {
        let mailer = SmtpMailer::new("smtp.example.test", 587, Some(("user", "hunter2")), "noreply@dasch.swiss")
            .expect("a valid relay should configure");
        let described = mailer.describe();
        assert!(described.contains("smtp.example.test:587"), "{described}");
        assert!(described.contains("noreply@dasch.swiss"), "{described}");
        assert!(
            !described.contains("hunter2"),
            "the password must never be in a log line: {described}"
        );
    }

    #[tokio::test]
    async fn test_the_console_transport_logs_the_body_and_not_the_recipient() {
        // REQ-6.8 with REQ-6.10: the message is logged so the service stays
        // usable without a relay, and the recipient is not part of it.
        //
        // The log is actually captured. The earlier version of this test only
        // checked that `send` returned `Ok` and that `describe()` mentioned the
        // variable — so adding `mail.to = %mail.to` to the event below would have
        // left it green, on the transport the PR preview actually runs.
        let mail = Mail {
            to: "someone@example.org".to_string(),
            subject: "Your sign-in code".to_string(),
            body: "Your code is 123456".to_string(),
        };

        let (logs, guard) = crate::test_support::capture_logs();
        assert_eq!(ConsoleMailer.send(&mail).await, Ok(()));
        drop(guard);

        let lines = logs.lines();
        assert!(
            lines.iter().any(|line| line.contains("123456")),
            "the code must reach the log, or the fallback delivers nothing: {lines:?}"
        );
        for line in &lines {
            assert!(!line.contains("someone@example.org"), "the recipient must not: {line}");
            assert!(!line.contains('@'), "and nothing address-shaped either: {line}");
        }
        assert!(ConsoleMailer.describe().contains("EDITOR_SMTP_HOST"));
    }
}
