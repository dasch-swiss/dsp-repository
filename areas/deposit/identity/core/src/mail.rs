//! What a transport is handed and what it may report back.
//!
//! An account holder's address must never reach a log or a trace, and an SMTP
//! failure is the one place such an address arrives from outside our own code: a
//! relay's reply routinely echoes the recipient, `550 5.1.1
//! <someone@example.org> User unknown` being the canonical shape. So
//! [`MailError`] carries a **classification and the three-digit status code**,
//! and never the reply text.
//!
//! The configured sender is a different matter and is logged on purpose: see
//! `SmtpMailer::describe` in `identity-store`. The argument is in
//! `docs/src/editor/authentication.md`.

use async_trait::async_trait;

/// One message, ready to hand to a transport.
pub struct Mail {
    /// The recipient. Never logged, never rendered.
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Why a message was not delivered.
///
/// Deliberately coarse. See the module docs: the relay's own words are the leak.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MailError {
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
pub trait Mailer: Send + Sync {
    async fn send(&self, mail: &Mail) -> Result<(), MailError>;

    /// How this transport is reported at startup, so an operator can see from
    /// the first log line whether mail actually leaves the process.
    fn describe(&self) -> String;
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
}
