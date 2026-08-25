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
        // Off by default: the reveal is the exception, so a test that wants it
        // has to say so, and every other test proves the ordinary path.
        reveal_login_code: false,
    };
    (state, mailer)
}

/// The whole app over `state`, as `serve()` assembles it.
///
/// Static assets come from a directory that does not exist: these tests are
/// about handlers and the route table, never about a file on disk.
pub(crate) fn test_app(state: &AppState) -> axum::Router {
    crate::router::build_app(state.clone(), "nonexistent-test-dir".as_ref())
}

/// Insert an account and return it.
pub(crate) async fn a_user(
    state: &AppState,
    email: &str,
    name: &str,
    role: editor_core::records::Role,
    shortcodes: &[&str],
) -> editor_core::records::User {
    use editor_core::repository::UserRepository;

    let user = editor_core::records::User {
        id: uuid::Uuid::new_v4(),
        email: email.to_string(),
        name: name.to_string(),
        role,
        shortcodes: shortcodes.iter().map(|s| (*s).to_string()).collect(),
        failed_logins: 0,
        failed_login_at: None,
        last_code_at: None,
        created_at: chrono::Utc::now(),
    };
    UserRepository::create(&state.db, &user)
        .await
        .expect("the fixture user should insert");
    user
}

/// A live session for `user_id`, as the session cookie's value.
///
/// Minted directly rather than by driving the login flow: a test about who may
/// reach a page should fail when authorization breaks, not when the mail
/// transport does.
pub(crate) async fn a_session(state: &AppState, user_id: uuid::Uuid) -> String {
    crate::auth::session::begin(&state.db, &state.auth, user_id, chrono::Utc::now())
        .await
        .expect("the fixture session should insert")
        .id
}

/// Rows in one table, so a test can assert on state no repository method
/// returns — what a rollback left behind, and what a cascade took.
pub(crate) async fn count_rows(db: &Database, table: &'static str) -> i64 {
    db.read(move |conn| conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)))
        .await
        .expect("counting rows should succeed")
}

/// Enough percent-encoding for the values these tests put in a form body.
///
/// Deliberately not a general encoder: it covers exactly the characters the
/// fixtures use. It lives here because the alternative was two copies escaping
/// different sets — the auth tests escaped `@` and `+`, the account tests also
/// `%`, space, comma and `/` — which is a difference that only ever shows up as
/// one suite passing on input the other would mangle.
pub(crate) fn urlencode(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('@', "%40")
        .replace(' ', "%20")
        .replace(',', "%2C")
        .replace('+', "%2B")
        .replace('/', "%2F")
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

thread_local! {
    /// Where this thread's capture, if any, collects. `None` on every thread
    /// that is not inside [`capture_logs`].
    static SINK: std::cell::RefCell<Option<LogCapture>> = const { std::cell::RefCell::new(None) };
}

fn to_sink(line: String) {
    SINK.with(|sink| {
        if let Some(capture) = sink.borrow().as_ref() {
            capture.push(line);
        }
    });
}

/// The one subscriber layer, installed globally and once. Routes each record to
/// the calling thread's sink, so tests on different threads cannot see each
/// other's output.
struct ThreadLocalCapture;

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for ThreadLocalCapture {
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        _id: &tracing::Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = TextVisitor(format!("span {}", attrs.metadata().name()));
        attrs.record(&mut visitor);
        to_sink(visitor.0);
    }

    fn on_record(
        &self,
        _id: &tracing::Id,
        values: &tracing::span::Record<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = TextVisitor("span-field".to_string());
        values.record(&mut visitor);
        to_sink(visitor.0);
    }

    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = TextVisitor(format!("event {} {}", event.metadata().level(), event.metadata().target()));
        event.record(&mut visitor);
        to_sink(visitor.0);
    }
}

/// Stops the capture and detaches this thread's sink.
pub(crate) struct CaptureGuard;

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        SINK.with(|sink| *sink.borrow_mut() = None);
    }
}

/// Capture everything logged on this thread until the guard is dropped.
///
/// ## Why this installs a *global* subscriber rather than a thread-local one
///
/// The obvious shape — `tracing::subscriber::set_default` per test — is racy in
/// a way that is invisible until it bites, and it bit: a test asserting a
/// refusal is logged passed sixty times alone and failed twice in forty runs of
/// the whole suite.
///
/// `tracing` caches each callsite's *interest* **process-globally**, while
/// `set_default` is thread-local. So when any test on any other thread reaches a
/// callsite while its own thread has no subscriber, that callsite is evaluated
/// against `NoSubscriber`, cached as `Interest::never()`, and then emits nothing
/// for **anyone** — including the thread that does have a capture installed.
/// `rebuild_interest_cache()` does not close it either: the poisoning can happen
/// again between the rebuild and the emit.
///
/// The failure mode is worse than flakiness for the tests asserting an address
/// is *absent*, because a capture that comes back empty passes. Those were
/// silently at risk of proving nothing.
///
/// So there is exactly one subscriber, installed once for the process, and it is
/// always enabled — no callsite can ever be cached as `never`. Routing to a
/// thread-local sink is what keeps parallel tests from seeing each other.
///
/// ## The sink is per-thread, so the runtime flavour still matters
///
/// A test has to stay on the thread that installed the sink. `#[tokio::test]`
/// gives a current-thread runtime by default, which is what makes that hold —
/// adding `flavor = "multi_thread"` lets the future resume on a thread with no
/// sink and the capture comes back empty. Nothing logged from a `deadpool`
/// `interact` closure is captured either, for the same reason.
///
/// An empty capture makes a "no address reached a log" assertion pass while
/// proving nothing, so **every negative test here pairs one with a positive
/// canary** — `assert!(!lines.is_empty())`, or an assertion that the account id
/// *is* present. That canary is what fails first if the sink ever detaches.
/// Keep the pairing on any new one.
pub(crate) fn capture_logs() -> (LogCapture, CaptureGuard) {
    use tracing_subscriber::layer::SubscriberExt;

    static INSTALLED: std::sync::Once = std::sync::Once::new();
    INSTALLED.call_once(|| {
        // Rebuilding the interest cache is what un-poisons any callsite already
        // evaluated against `NoSubscriber` — and it happens inside
        // `Dispatch::new`, not in `set_global_default` itself, which only stores
        // the dispatch. That same rebuild lifts the global max level off its
        // `OFF` default, so without it nothing would be recorded at all.
        //
        // `expect`, not a swallowed error: nothing else in this binary installs
        // a global subscriber, so a failure here means one appeared and every
        // capture is silently dead.
        tracing::subscriber::set_global_default(tracing_subscriber::registry().with(ThreadLocalCapture))
            .expect("no other global tracing subscriber should be installed in the test binary");
    });

    let capture = LogCapture::default();
    SINK.with(|sink| *sink.borrow_mut() = Some(capture.clone()));
    (capture, CaptureGuard)
}
