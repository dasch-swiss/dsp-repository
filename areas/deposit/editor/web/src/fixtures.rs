//! Test-only helpers shared by the editor's handler tests.
//!
//! Kept out of the modules under test so that a fake or a log capture is written once rather than
//! per test module, and so the fakes are as visible as the code they stand in for. Handler tests
//! sign in through `editor-ports`' fakes over an in-memory editor store: identity's own session
//! rules are not modelled here, and a test that depends on them belongs to the root.

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use axum::body::Body;
use axum::http::header::COOKIE;
use axum::http::{Request, Response};
use editor_core::repository::EditorRepositories;
use editor_ports::{FakeAuthenticator, FakeUserDirectory, Principal, Role};
use uuid::Uuid;

use crate::shell::EditorState;

/// The session cookie's name. The fake authenticator matches on the value alone; the real name is
/// kept so a request reads as the browser sends it.
pub(crate) const SESSION: &str = "__Host-editor_session";

#[derive(Clone)]
pub(crate) struct User {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) role: Role,
    pub(crate) assigned_shortcodes: Vec<String>,
}

/// [`EditorState`] plus the fakes it was built over, so a test can sign accounts in after the
/// state exists. Derefs to the state, so a handler or a router takes it as it would the real one.
#[derive(Clone)]
pub(crate) struct TestState {
    state: EditorState,
    authenticator: Arc<FakeAuthenticator>,
    directory: Arc<FakeUserDirectory>,
    users: Arc<Mutex<HashMap<Uuid, User>>>,
}

impl TestState {
    /// The same signed-in accounts over another store: the one a test wraps in `FaultyStore`
    /// while a session minted on `self` keeps working.
    pub(crate) fn with_store(&self, editor: Arc<dyn EditorRepositories>) -> TestState {
        let mut state = self.clone();
        state.state.editor = editor;
        state
    }
}

impl Deref for TestState {
    type Target = EditorState;

    fn deref(&self) -> &EditorState {
        &self.state
    }
}

impl DerefMut for TestState {
    fn deref_mut(&mut self) -> &mut EditorState {
        &mut self.state
    }
}

/// State over a fresh in-memory editor store.
///
/// The tuple keeps the shape of the call sites that bind a second value they ignore.
pub(crate) async fn test_state(label: &str) -> (TestState, ()) {
    let stores = open_test_db(label).await;
    (state_over(stores.editor), ())
}

/// The store of one test.
#[derive(Clone)]
pub(crate) struct Stores {
    pub(crate) editor: Arc<editor_store::Database>,
}

/// A fresh in-memory store for one test.
pub(crate) async fn open_test_db(label: &str) -> Stores {
    Stores {
        editor: Arc::new(editor_store::test_support::open_for_test(label).await),
    }
}

/// State over a store the caller built.
///
/// [`test_state`] covers every test that just needs a working store. This is for the two kinds
/// that need the handle as well: one that reads a table directly (`count_rows`: `EditorState` has
/// erased the concrete store behind the port), and one that puts the store's `FaultyStore` in front
/// of it while keeping the real store to seed and to assert against.
pub(crate) fn state_over(editor: Arc<dyn EditorRepositories>) -> TestState {
    let authenticator = Arc::new(FakeAuthenticator::new());
    let directory = Arc::new(FakeUserDirectory::new());
    TestState {
        state: crate::Editor::for_test(editor, authenticator.clone(), directory.clone()).state,
        authenticator,
        directory,
        users: Arc::default(),
    }
}

/// The editor's router over `state`, as `Editor::router` returns it: no CSRF layer and no root
/// fallback, which the root adds.
pub(crate) fn test_app(state: &TestState) -> axum::Router {
    crate::Editor { state: state.state.clone() }.router()
}

/// Register an account and return it. The address is not modelled: the editor never sees one. Its
/// display name goes into the directory, which is where the editor reads the names it shows.
pub(crate) async fn a_user(
    state: &TestState,
    _email: &str,
    name: &str,
    role: Role,
    assigned_shortcodes: &[&str],
) -> User {
    let user = User {
        id: Uuid::new_v4(),
        name: name.to_string(),
        role,
        assigned_shortcodes: assigned_shortcodes.iter().map(|s| (*s).to_string()).collect(),
    };
    state.directory.insert_account(&user.id.to_string(), &user.name);
    state
        .users
        .lock()
        .expect("the user map lock should not be poisoned")
        .insert(user.id, user.clone());
    user
}

/// A live session for the account `user_id`, as the session cookie's value.
///
/// Minted directly rather than by driving the login flow: a test about who may reach a page should
/// fail when authorization breaks, not when the mail transport does. The fake expires nothing, so
/// the deadline only has to be far enough out that no form shows the sign-out warning.
pub(crate) async fn a_session(state: &TestState, user_id: Uuid) -> String {
    let user = state
        .users
        .lock()
        .expect("the user map lock should not be poisoned")
        .get(&user_id)
        .cloned()
        .expect("a session is minted for an account `a_user` created");
    let token = Uuid::new_v4().to_string();
    state.authenticator.insert_session(
        &token,
        Principal {
            account_id: user.id.to_string(),
            display_name: user.name,
            role: user.role,
            assigned_shortcodes: user.assigned_shortcodes,
            signed_out_at: SystemTime::now() + Duration::from_secs(2 * 60 * 60),
        },
    );
    token
}

/// Enough percent-encoding for the values these tests put in a form body.
///
/// Deliberately not a general encoder: it covers exactly the characters the
/// fixtures use. `deposit-server`'s and `identity-web`'s fixtures hold copies; two escaping
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

/// The document `<title>`'s text, for a full page's body. Panics if there is
/// no `<title>` — every full page renders one.
pub(crate) fn title_of(body: &str) -> &str {
    let open = body.find("<title>").expect("a <title> tag") + "<title>".len();
    let close = body[open..].find("</title>").expect("a matching </title>");
    &body[open..open + close]
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
