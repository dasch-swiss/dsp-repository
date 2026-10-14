//! Test-only helpers shared by the router, csrf and cross-capability tests.
//!
//! Kept out of the modules under test so that a fake mailer or a log capture is
//! written once rather than per test module, and so the fakes are as visible as
//! the code they stand in for. The state here is the one place both capabilities'
//! real stores meet: identity's session rules and the editor's store, assembled
//! the way `serve()` assembles them. Handler tests that need neither live with
//! the handlers, in `editor-web` and `identity-web`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{Request, Response};
use editor_core::repository::EditorRepositories;
use identity_core::login::AuthConfig;
use identity_core::repository::IdentityRepositories;
pub(crate) use identity_web::test_support::RecordingMailer;

use crate::config::EditorConfig;

pub(crate) const CSS_HREF: &str = "/assets/app.css";

#[derive(Clone)]
pub(crate) struct IdentityFixture {
    pub(crate) store: Arc<dyn IdentityRepositories>,
    pub(crate) auth: AuthConfig,
    mailer: RecordingMailer,
}

/// Both capabilities' stores of one test, behind their ports, and what identity is assembled
/// from. [`test_app`] builds the whole app over it.
#[derive(Clone)]
pub(crate) struct TestState {
    pub(crate) identity: IdentityFixture,
    /// The editor's store, behind its ports, so a test can make a chosen storage call fail.
    pub(crate) editor: Arc<dyn EditorRepositories>,
    /// The live adapters over the same identity store, so a test signs in through identity's own
    /// session rules and a fault put in front of a store reaches the editor's reads too.
    pub(crate) authenticator: Arc<dyn editor_ports::Authenticator>,
    pub(crate) directory: Arc<dyn editor_ports::UserDirectory>,
}

/// State over fresh in-memory stores and a recording mailer.
///
/// The cooldown is zero because most tests are about something else and a real
/// one would make them sleep; the tests that are about the cooldown set it.
pub(crate) async fn test_state(label: &str) -> (TestState, RecordingMailer) {
    state_with(label, RecordingMailer::new(), |auth| auth.cooldown = Duration::ZERO).await
}

/// State with a chosen mailer and adjusted auth settings.
pub(crate) async fn state_with(
    label: &str,
    mailer: RecordingMailer,
    adjust: impl FnOnce(&mut AuthConfig),
) -> (TestState, RecordingMailer) {
    let stores = open_test_db(label).await;
    (state_over(stores.identity, stores.editor, mailer.clone(), adjust), mailer)
}

/// Both stores of one test, each a fresh in-memory database.
#[derive(Clone)]
pub(crate) struct Stores {
    pub(crate) identity: Arc<identity_store::Database>,
    pub(crate) editor: Arc<editor_store::Database>,
}

/// Fresh in-memory stores for one test.
pub(crate) async fn open_test_db(label: &str) -> Stores {
    Stores {
        identity: Arc::new(identity_store::test_support::open_for_test(label).await),
        editor: Arc::new(editor_store::test_support::open_for_test(label).await),
    }
}

/// State over stores the caller built.
///
/// [`state_with`] covers every test that just needs working stores. This is for the two kinds
/// that need the handles as well: one that reads a table directly (`count_rows` on either store:
/// `TestState` has erased the concrete stores behind the ports), and one that puts a store's
/// `FaultyStore` in front of it while keeping the real store to seed and to assert against.
pub(crate) fn state_over(
    identity: Arc<dyn IdentityRepositories>,
    editor: Arc<dyn EditorRepositories>,
    mailer: RecordingMailer,
    adjust: impl FnOnce(&mut AuthConfig),
) -> TestState {
    let mut auth = EditorConfig::default().identity_settings(String::new()).auth();
    adjust(&mut auth);
    TestState {
        authenticator: Arc::new(identity_store::LiveAuthenticator::new(identity.clone(), auth)),
        directory: Arc::new(identity_store::LiveUserDirectory::new(identity.clone())),
        identity: IdentityFixture { store: identity, auth, mailer },
        editor,
    }
}

/// The whole app over `state`, as `serve()` assembles it.
///
/// Static assets come from a directory that does not exist: these tests are
/// about handlers and the route table, never about a file on disk.
pub(crate) fn test_app(state: &TestState) -> axum::Router {
    crate::router::build_app(
        identity_router(state),
        editor_router(state),
        CSS_HREF.to_string(),
        "nonexistent-test-dir".as_ref(),
    )
}

pub(crate) fn identity_router(state: &TestState) -> axum::Router {
    identity_web::Identity::for_test(
        state.identity.store.clone(),
        Arc::new(editor_store::LiveAuthoredWork::new(state.editor.clone())),
        Arc::new(state.identity.mailer.clone()),
        state.identity.auth,
    )
    .router()
}

pub(crate) fn editor_router(state: &TestState) -> axum::Router {
    editor_web::Editor::for_test(state.editor.clone(), state.authenticator.clone(), state.directory.clone()).router()
}

/// Insert an account and return it.
pub(crate) async fn a_user(
    state: &TestState,
    email: &str,
    name: &str,
    role: identity_core::records::Role,
    shortcodes: &[&str],
) -> identity_core::records::User {
    use identity_core::repository::UserRepository;

    let user = identity_core::records::User {
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
    UserRepository::create(&*state.identity.store, &user)
        .await
        .expect("the fixture user should insert");
    user
}

/// A live session for `user_id`, as the session cookie's value.
///
/// Minted directly rather than by driving the login flow: a test about who may
/// reach a page should fail when authorization breaks, not when the mail
/// transport does.
pub(crate) async fn a_session(state: &TestState, user_id: uuid::Uuid) -> String {
    identity_core::session::begin(&*state.identity.store, &state.identity.auth, user_id, chrono::Utc::now())
        .await
        .expect("the fixture session should insert")
        .id
}

/// Enough percent-encoding for the values these tests put in a form body.
///
/// Deliberately not a general encoder: it covers exactly the characters the
/// fixtures use. `editor-web`'s and `identity-web`'s fixtures hold copies; two escaping
/// different sets shows up only as one suite passing on input the other would mangle, so a
/// character added here is added there too.
pub(crate) fn urlencode(value: &str) -> String {
    value
        // `%` first, or the escapes below would be escaped again.
        .replace('%', "%25")
        // `&` and `=` are the pair and name/value separators, so a value
        // holding either silently becomes two pairs or a renamed field. A
        // committed publication citation reads "in Di Natale A. & Basile C.",
        // which unescaped posts truncated at "Di Natale A." and then compares
        // the truncation against the file.
        .replace('&', "%26")
        .replace('=', "%3D")
        // `#` starts a fragment, which a server never receives.
        .replace('#', "%23")
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
/// **A global subscriber, not a thread-local one.** `tracing::subscriber::set_default` per test is
/// racy in a way that is invisible until it bites: `tracing` caches each callsite's *interest*
/// process-globally while `set_default` is thread-local, so when any test on any other thread
/// reaches a callsite while its own thread has no subscriber, that callsite is evaluated against
/// `NoSubscriber`, cached as `Interest::never()`, and then emits nothing for **anyone** —
/// including the thread that does have a capture installed. `rebuild_interest_cache()` does not
/// close it either: the poisoning can happen again between the rebuild and the emit. The failure
/// mode is worse than flakiness for a test asserting an address is *absent*, because a capture
/// that comes back empty passes.
///
/// So there is exactly one subscriber, installed once for the process and always enabled — no
/// callsite can ever be cached as `never` — and routing to a thread-local sink is what keeps
/// parallel tests from seeing each other's output.
///
/// **The sink is per-thread, so a test has to stay on the thread that installed it.**
/// `#[tokio::test]` gives a current-thread runtime by default, which is what makes that hold;
/// `flavor = "multi_thread"` lets the future resume on a thread with no sink and the capture comes
/// back empty. Nothing logged from a `deadpool` `interact` closure is captured either.
///
/// An empty capture makes a "no address reached a log" assertion pass while proving nothing, so
/// **every negative test here pairs one with a positive canary** — `assert!(!lines.is_empty())`,
/// or an assertion that the account id *is* present. Keep the pairing on any new one.
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
