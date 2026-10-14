//! Behind the `test-support` feature, so a production build cannot wire it in.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
pub use identity_core::cookie::{LOGIN, SESSION};
use identity_core::mail::{Mail, MailError, Mailer};

/// One message a [`RecordingMailer`] was asked to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentMail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// A mailer that remembers what it was given, and optionally refuses.
#[derive(Clone, Default)]
pub struct RecordingMailer {
    sent: Arc<Mutex<Vec<SentMail>>>,
    failure: Option<MailError>,
}

impl RecordingMailer {
    pub fn new() -> Self {
        Self::default()
    }

    /// A relay that always refuses — the path where the code rollback and the
    /// break-glass decision live.
    pub fn failing() -> Self {
        Self {
            sent: Arc::new(Mutex::new(Vec::new())),
            failure: Some(MailError::Refused { permanent: false, status: Some("451".to_string()) }),
        }
    }

    pub fn sent(&self) -> Vec<SentMail> {
        self.sent.lock().expect("the mailer lock should not be poisoned").clone()
    }

    /// The six digits out of the most recent message.
    pub fn last_code(&self) -> Option<String> {
        let sent = self.sent();
        let body = &sent.last()?.body;
        body.split_whitespace()
            .find(|word| word.len() == 6 && word.chars().all(|c| c.is_ascii_digit()))
            .map(str::to_string)
    }
}

#[async_trait]
impl Mailer for RecordingMailer {
    async fn send(&self, mail: &Mail) -> Result<(), MailError> {
        // Recorded even when the send fails, so a test can assert what would
        // have gone out and to whom.
        self.sent
            .lock()
            .expect("the mailer lock should not be poisoned")
            .push(SentMail {
                to: mail.to.clone(),
                subject: mail.subject.clone(),
                body: mail.body.clone(),
            });
        match &self.failure {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    fn describe(&self) -> String {
        "recording (test)".to_string()
    }
}
