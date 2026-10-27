//! Sending the one-time code over a real relay or the log, and what happens
//! when there is no relay or the relay refuses.
//!
//! The transports report failure as an [`identity_core::mail::MailError`], which
//! carries a classification and never the relay's reply text: a reply routinely
//! echoes the recipient. `classify` is where an SMTP error is reduced to one.
//!
//! The configured sender is a different matter and is logged on purpose: see
//! `SmtpMailer::describe`. The argument is in
//! `docs/src/editor/authentication.md`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use identity_core::mail::{Mail, MailError, Mailer};

/// The transport for this configuration: the SMTP relay at `smtp_host`, or, when there is none,
/// the console transport, which writes each message to the log.
///
/// Fails when a relay is configured but cannot be, so a misconfigured relay or an unusable DKIM
/// key stops startup rather than the first login.
pub fn mailer(
    smtp_host: Option<&str>,
    port: u16,
    credentials: Option<(&str, &str)>,
    dkim: Option<(&str, &Path)>,
    from: &str,
) -> Result<Arc<dyn Mailer>, SmtpSetupError> {
    match smtp_host {
        Some(host) => Ok(Arc::new(SmtpMailer::new(host, port, credentials, dkim, from)?)),
        None => Ok(Arc::new(ConsoleMailer)),
    }
}

/// No relay configured, so the message goes to the log and the service stays usable.
///
/// This is the development and PR-preview transport, and it is the break-glass
/// for a broken relay: unsetting `EDITOR_SMTP_HOST` routes every code here.
pub(crate) struct ConsoleMailer;

#[async_trait]
impl Mailer for ConsoleMailer {
    async fn send(&self, mail: &Mail) -> Result<(), MailError> {
        // The recipient is deliberately absent. Whoever is reading
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

/// The configured SMTP relay, over STARTTLS.
pub(crate) struct SmtpMailer {
    transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
    from: lettre::message::Mailbox,
    host: String,
    port: u16,
    // `DkimConfig` is `Debug` over the private key, which is why this struct has no `Debug`.
    // The selector is kept beside it because `DkimConfig`'s fields are private.
    dkim: Option<(lettre::message::dkim::DkimConfig, String)>,
}

/// A relay that could not be configured at all, which stops startup rather than
/// failing at the first login.
///
/// `DkimKey` never forwards a parser's error, unlike the TLS setup case argued in
/// `SmtpMailer::new`: a key parser's message can quote the key, and this error is logged.
#[derive(Debug, thiserror::Error)]
pub enum SmtpSetupError {
    #[error("EDITOR_SMTP_FROM is not a valid mailbox: {0}")]
    From(String),
    #[error("the SMTP relay could not be configured: {0}")]
    Relay(String),
    #[error("EDITOR_DKIM_PRIVATE_KEY_FILE {}: {reason}", path.display())]
    DkimKey { path: PathBuf, reason: String },
}

impl SmtpMailer {
    /// Build the transport. Does not connect — a relay that is down must not
    /// stop the process, only fail the sends it is asked for. It does read the DKIM key file.
    pub(crate) fn new(
        host: &str,
        port: u16,
        credentials: Option<(&str, &str)>,
        dkim: Option<(&str, &Path)>,
        from: &str,
    ) -> Result<Self, SmtpSetupError> {
        use lettre::message::dkim::{DkimCanonicalization, DkimCanonicalizationType, DkimConfig};
        use lettre::message::header::HeaderName;
        use lettre::transport::smtp::authentication::Credentials;

        let from: lettre::message::Mailbox = from.parse().map_err(|e| SmtpSetupError::From(format!("{e}")))?;

        // `d=` is the `From:` domain by construction, so the signature aligns with it, which
        // DMARC's `adkim=s` needs. Every header listed must be on the message: lettre cannot
        // oversign, and `message` sets the `Message-ID`.
        let dkim = dkim
            .map(|(selector, key_file)| {
                let headers = ["From", "To", "Subject", "Date", "Message-ID", "Content-Type"]
                    .map(HeaderName::new_from_ascii_str)
                    .to_vec();
                let config = DkimConfig::new(
                    selector.to_string(),
                    from.email.domain().to_string(),
                    load_dkim_key(key_file)?,
                    headers,
                    DkimCanonicalization {
                        header: DkimCanonicalizationType::Relaxed,
                        body: DkimCanonicalizationType::Relaxed,
                    },
                );
                Ok((config, selector.to_string()))
            })
            .transpose()?;

        // STARTTLS (RFC 2487) on whatever port the relay takes; `starttls_relay` still requires
        // the upgrade to succeed, so this is not an opportunistic downgrade path.
        //
        // `format!("{e}")` on an SMTP error is the operation the `identity_core::mail`
        // module docs forbid: `Display` on `lettre::transport::smtp::Error` appends its
        // source, and for a *connected* transport that source is the relay's
        // reply — which quotes the recipient. It is safe only here, because
        // `starttls_relay`'s one fallible step is `TlsParameters::new(host)`: no
        // message, no envelope, no connection and no reply, and credentials are
        // attached after this call.
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
            dkim,
        })
    }

    /// The message as it goes to the relay: `Message-ID` set and, when configured, DKIM-signed.
    fn message(&self, mail: &Mail) -> Result<lettre::Message, MailError> {
        use lettre::message::header::ContentType;

        let to: lettre::message::Mailbox = mail.to.parse().map_err(|_| MailError::Malformed)?;
        let message_id = format!("<{}@{}>", uuid::Uuid::new_v4(), self.from.email.domain());
        let mut message = lettre::Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(&mail.subject)
            .message_id(Some(message_id))
            .header(ContentType::TEXT_PLAIN)
            .body(mail.body.clone())
            .map_err(|_| MailError::Malformed)?;
        if let Some((config, _)) = &self.dkim {
            message.sign(config);
        }
        Ok(message)
    }
}

/// The signing key at `path`, in PKCS#8 (what `openssl genrsa` writes on OpenSSL 3) or PKCS#1.
///
/// No error carries the parser's message or the file's bytes: `Identity::start`'s failure is
/// logged, and the log is a pipeline other people read.
fn load_dkim_key(path: &Path) -> Result<lettre::message::dkim::DkimSigningKey, SmtpSetupError> {
    use lettre::message::dkim::{DkimSigningAlgorithm, DkimSigningKey};
    use rsa::pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey, LineEnding};
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::traits::PublicKeyParts;

    let refused = |reason: String| SmtpSetupError::DkimKey { path: path.to_path_buf(), reason };
    let pem = std::fs::read_to_string(path).map_err(|e| refused(e.kind().to_string()))?;
    let key = rsa::RsaPrivateKey::from_pkcs8_pem(&pem)
        .or_else(|_| rsa::RsaPrivateKey::from_pkcs1_pem(&pem))
        .map_err(|_| refused("not an RSA private key in PKCS#8 or PKCS#1 PEM".to_string()))?;
    // `n().bits()`, not `size() * 8`: `size()` rounds up to whole bytes, so a 1017-bit modulus
    // would pass as 1024.
    let bits = key.n().bits();
    if bits < 1024 {
        return Err(refused(format!("{bits}-bit key; DKIM needs at least 1024")));
    }
    // `rsa` applies its 4096-bit `MAX_SIZE` to public keys only, so a decoded private key can be
    // larger, and DKIM verifiers are not required to accept one (RFC 8301 §3.2).
    if bits > 4096 {
        return Err(refused(format!("{bits}-bit key; DKIM verifiers need at most 4096")));
    }
    let pkcs1 = key
        .to_pkcs1_pem(LineEnding::LF)
        .map_err(|_| refused("could not re-encode the key".to_string()))?;
    DkimSigningKey::new(&pkcs1, DkimSigningAlgorithm::Rsa).map_err(|_| refused("lettre rejected the key".to_string()))
}

/// Reduce a transport error to what may be logged.
///
/// See the `identity_core::mail` module docs: `error.to_string()` on an SMTP error can contain the
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
        use lettre::AsyncTransport;

        let message = self.message(mail)?;
        self.transport.send(message).await.map_err(|e| classify(&e))?;
        Ok(())
    }

    fn describe(&self) -> String {
        let signing = match &self.dkim {
            Some((_, selector)) => format!("DKIM d={} s={selector}", self.from.email.domain()),
            None => "unsigned".to_string(),
        };
        format!("SMTP {}:{} (STARTTLS) as {} {signing}", self.host, self.port, self.from.email)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_an_invalid_from_address_stops_startup() {
        // Reported at startup rather than at the first login: an unparseable
        // `EDITOR_SMTP_FROM` makes every send fail, and finding that out from a
        // user report is the expensive way.
        // Matched rather than unwrapped: the transport itself is not `Debug`,
        // deliberately — it holds credentials.
        let result = SmtpMailer::new("smtp.example.test", 587, None, None, "not a mailbox");
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
        let mailer = SmtpMailer::new("smtp.example.test", 587, Some(("user", "hunter2")), None, "noreply@dasch.swiss")
            .expect("a valid relay should configure");
        let described = mailer.describe();
        assert!(described.contains("smtp.example.test:587"), "{described}");
        assert!(described.contains("noreply@dasch.swiss"), "{described}");
        assert!(described.contains("unsigned"), "{described}");
        assert!(
            !described.contains("hunter2"),
            "the password must never be in a log line: {described}"
        );
    }

    #[tokio::test]
    async fn test_a_pkcs8_key_signs_as_the_from_domain_with_relaxed_canonicalization() {
        let key = key_file(&pkcs8_pem(rsa_key()));
        let mailer = SmtpMailer::new(
            "smtp.example.test",
            25,
            None,
            Some(("dasch-test-2026", key.path())),
            "noreply@dasch.swiss",
        )
        .expect("a valid key should configure");
        let mail = Mail {
            to: "someone@example.org".to_string(),
            subject: "Your sign-in code".to_string(),
            body: "Your code is 123456".to_string(),
        };

        let formatted =
            String::from_utf8(mailer.message(&mail).expect("a valid mail").formatted()).expect("the message is ASCII");
        // lettre folds long headers.
        let unfolded = formatted.replace("\r\n ", " ").replace("\r\n\t", " ");

        assert_eq!(unfolded.matches("DKIM-Signature:").count(), 1, "{unfolded}");
        let signature = unfolded
            .lines()
            .find(|line| line.starts_with("DKIM-Signature:"))
            .expect("one signature");
        assert!(signature.contains("d=dasch.swiss;"), "{signature}");
        assert!(signature.contains("s=dasch-test-2026;"), "{signature}");
        assert!(signature.contains("c=relaxed/relaxed;"), "{signature}");
        assert!(
            signature.contains("h=from:to:subject:date:message-id:content-type;"),
            "{signature}"
        );
        let message_id = unfolded
            .lines()
            .find(|line| line.starts_with("Message-ID:"))
            .expect("a Message-ID");
        assert!(message_id.ends_with("@dasch.swiss>"), "{message_id}");

        let described = mailer.describe();
        assert!(described.contains("DKIM d=dasch.swiss s=dasch-test-2026"), "{described}");
    }

    #[tokio::test]
    async fn test_a_pkcs1_key_loads() {
        use rsa::pkcs1::{EncodeRsaPrivateKey, LineEnding};

        let pem = rsa_key().to_pkcs1_pem(LineEnding::LF).expect("encodes");
        assert!(pem.contains("BEGIN RSA PRIVATE KEY"), "this must be the PKCS#1 form");
        let key = key_file(&pem);
        let mailer = SmtpMailer::new(
            "smtp.example.test",
            25,
            None,
            Some(("dasch-test-2026", key.path())),
            "noreply@dasch.swiss",
        );
        assert!(mailer.is_ok(), "a PKCS#1 key must load");
    }

    #[tokio::test]
    async fn test_an_unusable_key_stops_startup_naming_the_path_never_the_content() {
        use rand::thread_rng;

        const MARKER: &str = "s3cr3t-marker-9f2c";
        let fake = key_file(&format!("-----BEGIN PRIVATE KEY-----\n{MARKER}\n-----END PRIVATE KEY-----\n"));
        let too_small = key_file(&pkcs8_pem(
            &rsa::RsaPrivateKey::new(&mut thread_rng(), 512).expect("a 512-bit key generates"),
        ));
        let missing = fake.path().with_file_name("no-such-key.pem");

        for (path, reason) in [
            (missing.as_path(), "not found"),
            (fake.path(), "not an RSA private key"),
            (too_small.path(), "512-bit key"),
        ] {
            let result = SmtpMailer::new(
                "smtp.example.test",
                25,
                None,
                Some(("dasch-test-2026", path)),
                "noreply@dasch.swiss",
            );
            let Some(error @ SmtpSetupError::DkimKey { .. }) = result.err() else {
                panic!("{} must be refused as a DKIM key error", path.display());
            };
            let rendered = error.to_string();
            assert!(rendered.contains(&path.display().to_string()), "{rendered}");
            assert!(rendered.contains(reason), "{rendered}");
            assert!(!rendered.contains(MARKER), "{rendered}");
            assert!(!rendered.contains("PRIVATE KEY"), "{rendered}");
        }
    }

    #[tokio::test]
    #[ignore = "generates a 4104-bit key, 20 to 60 s in a debug build; run with `--ignored`"]
    async fn test_a_key_over_4096_bits_stops_startup() {
        let key = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 4104).expect("a 4104-bit key generates");
        let file = key_file(&pkcs8_pem(&key));
        let result = SmtpMailer::new(
            "smtp.example.test",
            25,
            None,
            Some(("dasch-test-2026", file.path())),
            "noreply@dasch.swiss",
        );
        let Some(error @ SmtpSetupError::DkimKey { .. }) = result.err() else {
            panic!("a 4104-bit key must be refused as a DKIM key error");
        };
        assert!(error.to_string().contains("4104-bit key"), "{error}");
    }

    /// One key per test binary: generating an RSA key in a debug build takes seconds.
    fn rsa_key() -> &'static rsa::RsaPrivateKey {
        static KEY: std::sync::OnceLock<rsa::RsaPrivateKey> = std::sync::OnceLock::new();
        KEY.get_or_init(|| rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 1024).expect("a 1024-bit key generates"))
    }

    fn pkcs8_pem(key: &rsa::RsaPrivateKey) -> String {
        use rsa::pkcs8::{EncodePrivateKey, LineEnding};

        key.to_pkcs8_pem(LineEnding::LF).expect("encodes").to_string()
    }

    fn key_file(pem: &str) -> tempfile::NamedTempFile {
        use std::io::Write;

        let mut file = tempfile::NamedTempFile::new().expect("a temp file");
        file.write_all(pem.as_bytes()).expect("written");
        file
    }
}
