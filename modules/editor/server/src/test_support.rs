//! Test-only helpers shared by the router, auth and account tests.
//!
//! Kept out of the modules under test so that a fake mailer or a log capture is
//! written once rather than per test module, and so the fakes are as visible as
//! the code they stand in for.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{Request, Response};

use crate::auth::AuthConfig;
use crate::config::EditorConfig;
use crate::db::{Database, Source};
use crate::mail::{Mail, MailError, Mailer};
use crate::AppState;

/// One message a [`RecordingMailer`] was asked to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SentMail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// A mailer that remembers what it was given, and optionally refuses.
#[derive(Clone, Default)]
pub(crate) struct RecordingMailer {
    sent: Arc<Mutex<Vec<SentMail>>>,
    failure: Option<MailError>,
}

impl RecordingMailer {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A relay that always refuses — the path where the code rollback and the
    /// break-glass decision live.
    pub(crate) fn failing() -> Self {
        Self {
            sent: Arc::new(Mutex::new(Vec::new())),
            failure: Some(MailError::Refused { permanent: false, status: Some("451".to_string()) }),
        }
    }

    pub(crate) fn sent(&self) -> Vec<SentMail> {
        self.sent.lock().expect("the mailer lock should not be poisoned").clone()
    }

    /// The six digits out of the most recent message.
    pub(crate) fn last_code(&self) -> Option<String> {
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

/// App state over a fresh in-memory database and a recording mailer.
///
/// The cooldown is zero because most tests are about something else and a real
/// one would make them sleep; the tests that are about the cooldown set it.
pub(crate) async fn test_state(label: &str) -> (AppState, RecordingMailer) {
    state_with(label, RecordingMailer::new(), |auth| auth.cooldown = Duration::ZERO).await
}

/// App state with a chosen mailer and adjusted auth settings.
pub(crate) async fn state_with(
    label: &str,
    mailer: RecordingMailer,
    adjust: impl FnOnce(&mut AuthConfig),
) -> (AppState, RecordingMailer) {
    let db = Database::open(Source::memory_for_test(label), 4, Duration::from_secs(5))
        .await
        .expect("the test database should open");
    let mut auth = AuthConfig::from(&EditorConfig::default());
    adjust(&mut auth);
    let state = AppState {
        css_href: "/assets/app.css".to_string(),
        db,
        mailer: Arc::new(mailer.clone()),
        auth,
    };
    (state, mailer)
}

/// Rows in one table, so a test can assert on state no repository method
/// returns — what a rollback left behind, and what a cascade took.
pub(crate) async fn count_rows(db: &Database, table: &'static str) -> i64 {
    db.read(move |conn| conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)))
        .await
        .expect("counting rows should succeed")
}

/// A `GET` as a browser sends it.
pub(crate) fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header("x-forwarded-for", "203.0.113.7")
        .body(Body::empty())
        .expect("the request should build")
}

/// A form `POST` as a browser sends it, `Sec-Fetch-Site` included — without it
/// the CSRF middleware refuses before any handler runs, which is the point of
/// that middleware and a trap for every test that forgets.
pub(crate) fn post(uri: &str, form: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("x-forwarded-for", "203.0.113.7")
        .header("sec-fetch-site", "same-origin")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(form.to_string()))
        .expect("the request should build")
}

/// Attach a cookie to a request.
pub(crate) fn with_cookie(mut request: Request<Body>, name: &str, value: &str) -> Request<Body> {
    request
        .headers_mut()
        .append(COOKIE, format!("{name}={value}").parse().expect("a cookie header should build"));
    request
}

/// The value a response sets for `name`, or `None` if it sets no such cookie.
///
/// Returns the empty string for a cookie being cleared, which is how a test
/// tells "cleared" from "left alone".
pub(crate) fn cookie_set<T>(response: &Response<T>, name: &str) -> Option<String> {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| value.strip_prefix(&format!("{name}=")))
        .map(|rest| rest.split(';').next().unwrap_or("").to_string())
}

/// The `Location` a redirect points at.
pub(crate) fn location<T>(response: &Response<T>) -> Option<String> {
    response
        .headers()
        .get(axum::http::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

/// The response body as a string.
pub(crate) async fn body_string(response: Response<Body>) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 1_000_000)
        .await
        .expect("the body should read");
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Every `tracing` event and span field emitted on this thread, as text.
///
/// Spans are captured as well as events: `#[tracing::instrument]` records its
/// arguments as span fields, so a capture that only saw events would pass the
/// no-address-in-logs test while an address sat in a span attribute.
#[derive(Clone, Default)]
pub(crate) struct LogCapture(Arc<Mutex<Vec<String>>>);

impl LogCapture {
    pub(crate) fn lines(&self) -> Vec<String> {
        self.0.lock().expect("the log lock should not be poisoned").clone()
    }

    fn push(&self, line: String) {
        self.0.lock().expect("the log lock should not be poisoned").push(line);
    }
}

#[derive(Default)]
struct TextVisitor(String);

impl tracing::field::Visit for TextVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        use std::fmt::Write;
        let _ = write!(self.0, " {}={value:?}", field.name());
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        use std::fmt::Write;
        let _ = write!(self.0, " {}={value}", field.name());
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for LogCapture {
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        _id: &tracing::Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = TextVisitor(format!("span {}", attrs.metadata().name()));
        attrs.record(&mut visitor);
        self.push(visitor.0);
    }

    fn on_record(
        &self,
        _id: &tracing::Id,
        values: &tracing::span::Record<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = TextVisitor("span-field".to_string());
        values.record(&mut visitor);
        self.push(visitor.0);
    }

    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = TextVisitor(format!("event {} {}", event.metadata().level(), event.metadata().target()));
        event.record(&mut visitor);
        self.push(visitor.0);
    }
}

/// Capture everything logged on this thread until the guard is dropped.
///
/// `set_default` is thread-local, which is why these tests run on the
/// single-threaded runtime `#[tokio::test]` gives by default: on a multi-thread
/// runtime the future could resume on a thread with no subscriber and the
/// capture would silently come back empty.
pub(crate) fn capture_logs() -> (LogCapture, tracing::subscriber::DefaultGuard) {
    use tracing_subscriber::layer::SubscriberExt;

    let capture = LogCapture::default();
    let subscriber = tracing_subscriber::registry().with(capture.clone());
    let guard = tracing::subscriber::set_default(subscriber);
    (capture, guard)
}
