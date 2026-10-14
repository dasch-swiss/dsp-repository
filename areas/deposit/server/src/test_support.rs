//! Test-only helpers shared by the router, csrf and cross-capability tests.
//!
//! Kept out of the modules under test so that a fake mailer is written once rather
//! than per test module, and so the fakes are as visible as the code they stand in
//! for. The state here is the one place both capabilities' real stores meet:
//! identity's session rules and the editor's store, assembled the way `serve()`
//! assembles them. Handler tests that need neither live with the handlers, in
//! `editor-web` and `identity-web`.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{Request, Response};
pub(crate) use identity_web::test_support::RecordingMailer;
use identity_web::IdentitySettings;

use crate::config::EditorConfig;

pub(crate) const CSS_HREF: &str = "/assets/app.css";

#[derive(Clone)]
pub(crate) struct IdentityFixture {
    pub(crate) store: Arc<identity_store::Database>,
    pub(crate) settings: IdentitySettings,
    mailer: RecordingMailer,
}

/// Both capabilities' stores of one test, and what identity is assembled from. [`test_app`] builds
/// the whole app over it.
#[derive(Clone)]
pub(crate) struct TestState {
    pub(crate) identity: IdentityFixture,
    pub(crate) editor: Arc<editor_store::Database>,
    /// The live adapters over the same identity store, so a test signs in through identity's own
    /// session rules.
    pub(crate) authenticator: Arc<dyn editor_ports::Authenticator>,
    pub(crate) directory: Arc<dyn editor_ports::UserDirectory>,
}

/// State over fresh in-memory stores and a recording mailer.
///
/// The cooldown is zero because most tests are about something else and a real
/// one would make them sleep; the tests that are about the cooldown set it.
pub(crate) async fn test_state(label: &str) -> (TestState, RecordingMailer) {
    state_with(label, RecordingMailer::new(), |settings| {
        settings.login_cooldown = Duration::ZERO
    })
    .await
}

/// State with a chosen mailer and adjusted auth settings.
pub(crate) async fn state_with(
    label: &str,
    mailer: RecordingMailer,
    adjust: impl FnOnce(&mut IdentitySettings),
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
/// [`state_with`] covers every test that just needs working stores. This is for the tests that
/// need the handles as well, to seed through the stores' own helpers or to read a table directly.
pub(crate) fn state_over(
    identity: Arc<identity_store::Database>,
    editor: Arc<editor_store::Database>,
    mailer: RecordingMailer,
    adjust: impl FnOnce(&mut IdentitySettings),
) -> TestState {
    let mut settings = EditorConfig::default().identity_settings(String::new());
    adjust(&mut settings);
    TestState {
        authenticator: Arc::new(identity_store::LiveAuthenticator::new(identity.clone(), settings.auth())),
        directory: Arc::new(identity_store::LiveUserDirectory::new(identity.clone())),
        identity: IdentityFixture { store: identity, settings, mailer },
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
        state.identity.settings.auth(),
    )
    .router()
}

pub(crate) fn editor_router(state: &TestState) -> axum::Router {
    editor_web::Editor::for_test(state.editor.clone(), state.authenticator.clone(), state.directory.clone()).router()
}

/// Insert an account and return its id. `role` is `"depositor"` or `"rdu"`.
pub(crate) async fn a_user(state: &TestState, email: &str, name: &str, role: &str, shortcodes: &[&str]) -> String {
    identity_store::test_support::seed_user(&state.identity.store, email, name, role, shortcodes).await
}

/// A live session for `user_id`, as the session cookie's value.
///
/// Minted directly rather than by driving the login flow: a test about who may
/// reach a page should fail when authorization breaks, not when the mail
/// transport does.
pub(crate) async fn a_session(state: &TestState, user_id: &str) -> String {
    let now = chrono::Utc::now();
    let lifetime = chrono::TimeDelta::from_std(state.identity.settings.session_absolute)
        .expect("the session lifetime should fit a duration");
    identity_store::test_support::seed_session(&state.identity.store, user_id, now, now + lifetime).await
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
