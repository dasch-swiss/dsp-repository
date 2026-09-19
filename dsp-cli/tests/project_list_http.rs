// `reqwest::blocking` is safe alongside `#[tokio::test]` (which wiremock
// requires) provided the blocking client is constructed, used, and dropped
// entirely on a plain OS thread — not on the tokio runtime's thread pool.
// We achieve this with `std::thread::spawn` + `JoinHandle::join`: the
// blocking reqwest runtime lives and dies on its own OS thread, so it never
// tries to drop a Tokio runtime from within an async context (which would
// panic). Do not "fix" this by moving the `HttpDspClient::new()` call back
// into the async body without also dropping the blocking runtime on a
// non-async thread.
//
// The `GET /admin/projects` endpoint is public: an unauthenticated call (token
// = None) omits the Authorization header entirely; an authenticated call sends
// `Authorization: Bearer <token>`. Tests assert both behaviours so a future
// refactor can't accidentally start sending an unconditional empty bearer.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::ProjectStatus;
use serde_json::json;
use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// A clearly synthetic token — never a real JWT.
const TOKEN: &str = "test-token";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal projects-list API response body.
fn projects_body(projects: serde_json::Value) -> serde_json::Value {
    json!({ "projects": projects })
}

// ---------------------------------------------------------------------------
// Happy path — translates DTO correctly
// ---------------------------------------------------------------------------

/// A 200 response with a mixed fixture (active + inactive, with/without ontologies,
/// with/without longname) is translated to the correct `Vec<Project>`.
///
/// Assertions cover:
/// - `status: true`  → `ProjectStatus::Active`
/// - `status: false` → `ProjectStatus::Inactive`
/// - `ontologies: ["o1", "o2"]` → `data_models: 2`
/// - `ontologies: []` (explicit empty) → `data_models: 0`
/// - `longname: null` (absent from JSON) → `longname: None`
/// - `longname: "Some Name"` → `longname: Some("Some Name")`
/// - `id` field maps to `iri`
#[tokio::test]
async fn happy_path_translates_dto_to_project() {
    let server = MockServer::start().await;

    let body = projects_body(json!([
        {
            "id": "http://rdfh.ch/projects/0001",
            "shortcode": "0001",
            "shortname": "anything",
            "longname": "Anything Project",
            "status": true,
            "ontologies": ["http://www.knora.org/ontology/0001/anything", "http://www.knora.org/ontology/0001/minimal"]
        },
        {
            "id": "http://rdfh.ch/projects/0002",
            "shortcode": "0002",
            "shortname": "images",
            // longname absent (null) — must map to None
            "status": false,
            "ontologies": []
        },
        {
            "id": "http://rdfh.ch/projects/0803",
            "shortcode": "0803",
            "shortname": "incunabula",
            "longname": "Incunabula Project",
            "status": true,
            "ontologies": ["http://www.knora.org/ontology/0803/incunabula"]
        }
    ]));

    Mock::given(method("GET"))
        .and(path("/admin/projects"))
        .and(header("user-agent", format!("dsp-cli/{}", env!("CARGO_PKG_VERSION")).as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_projects(&uri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let projects = result.unwrap();
    assert_eq!(projects.len(), 3, "expected 3 projects");

    // Project 1: active, 2 data_models, longname present
    let p1 = &projects[0];
    assert_eq!(p1.iri, "http://rdfh.ch/projects/0001");
    assert_eq!(p1.shortcode, "0001");
    assert_eq!(p1.shortname, "anything");
    assert_eq!(p1.longname.as_deref(), Some("Anything Project"));
    assert_eq!(p1.status, ProjectStatus::Active, "status:true must be Active");
    assert_eq!(p1.data_models, 2, "two ontologies → data_models: 2");

    // Project 2: inactive, 0 data_models, no longname
    let p2 = &projects[1];
    assert_eq!(p2.iri, "http://rdfh.ch/projects/0002");
    assert_eq!(p2.shortcode, "0002");
    assert_eq!(p2.shortname, "images");
    assert_eq!(p2.longname, None, "absent longname must map to None");
    assert_eq!(p2.status, ProjectStatus::Inactive, "status:false must be Inactive");
    assert_eq!(p2.data_models, 0, "empty ontologies → data_models: 0");

    // Project 3: active, 1 data_model, longname present
    let p3 = &projects[2];
    assert_eq!(p3.iri, "http://rdfh.ch/projects/0803");
    assert_eq!(p3.data_models, 1);
    assert_eq!(p3.status, ProjectStatus::Active, "second active project must also be Active");
}

// ---------------------------------------------------------------------------
// Server error path
// ---------------------------------------------------------------------------

/// A 500 response maps to `Diagnostic::ServerError`.
#[tokio::test]
async fn server_error_500_returns_server_error_diagnostic() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/projects"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_projects(&uri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 500 response");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "500 must map to Diagnostic::ServerError"
    );
}

// ---------------------------------------------------------------------------
// Bearer-PRESENT assertion
// ---------------------------------------------------------------------------

/// When `token = Some("test-token")` is passed, the request carries an
/// `Authorization: Bearer test-token` header. This guards against a future
/// refactor that drops the token pass-through.
#[tokio::test]
async fn bearer_present_when_token_is_some() {
    let server = MockServer::start().await;

    // Only match requests that carry the correct Authorization header.
    Mock::given(method("GET"))
        .and(path("/admin/projects"))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(projects_body(json!([]))))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_projects(&uri, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok when bearer token is forwarded, got: {:?}", result);

    // Also inspect recorded headers directly.
    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    let auth = received[0]
        .headers
        .get("authorization")
        .expect("Authorization header must be present when token is Some");
    assert_eq!(
        auth.to_str().expect("header should be valid UTF-8"),
        format!("Bearer {TOKEN}"),
        "bearer token must match the value passed to list_projects"
    );
}

// ---------------------------------------------------------------------------
// Bearer-ABSENT assertion
// ---------------------------------------------------------------------------

/// When `token = None` is passed, the request has NO `Authorization` header.
/// This prevents a future refactor from accidentally sending an empty or
/// unconditional bearer (which would change the request semantics for the
/// public endpoint).
#[tokio::test]
async fn bearer_absent_when_token_is_none() {
    let server = MockServer::start().await;

    // Match only requests that do NOT have an Authorization header.
    Mock::given(method("GET"))
        .and(path("/admin/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(projects_body(json!([]))))
        .mount(&server)
        .await;

    let uri = server.uri();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_projects(&uri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    // Inspect recorded headers to confirm the Authorization header is absent.
    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    assert!(
        received[0].headers.get("authorization").is_none(),
        "Authorization header must be ABSENT when token is None; \
         a future refactor must not start sending an unconditional bearer"
    );
}

// ---------------------------------------------------------------------------
// Parse failure — missing `status` field → ServerError
// ---------------------------------------------------------------------------

/// A response where a project item is missing the `status` field must fail
/// parse loudly (`ServerError`), not silently default to `false`.
/// `status` has no `#[serde(default)]` by design — see the DTO doc-comment.
#[tokio::test]
async fn missing_status_field_returns_server_error() {
    let server = MockServer::start().await;

    // `status` key is deliberately absent — missing required field.
    let body = projects_body(json!([
        {
            "id": "http://rdfh.ch/projects/0001",
            "shortcode": "0001",
            "shortname": "anything"
            // "status" is absent — must fail parse
        }
    ]));

    Mock::given(method("GET"))
        .and(path("/admin/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_projects(&uri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_err(),
        "missing `status` field must cause a parse error, not silently default"
    );
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "parse failure must map to Diagnostic::ServerError"
    );
}

// ---------------------------------------------------------------------------
// `header_exists` negative check (alternative matcher approach for ABSENT)
// ---------------------------------------------------------------------------

/// Complementary test using wiremock's `header_exists` negative logic:
/// mount a mock that only responds when the Authorization header IS present,
/// then verify with `token = None` that wiremock reports the request did NOT
/// match the header-gated mock (i.e. zero requests matched with auth header).
///
/// This is the "future-refactor canary" — if `list_projects(None)` starts
/// sending a bearer, the header-absent mock above will still succeed (wiremock
/// routes to the fallback), but `received_requests` will show the header,
/// catching the regression in `bearer_absent_when_token_is_none`.
/// The test below acts as a second line of defence using a wiremock expectation.
#[tokio::test]
async fn no_token_does_not_match_bearer_gated_mock() {
    let server = MockServer::start().await;

    // Mount a mock that ONLY matches requests with an Authorization header and
    // expects exactly 0 hits. If list_projects(None) sends a bearer this
    // expectation will fail at drop time.
    Mock::given(method("GET"))
        .and(path("/admin/projects"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(projects_body(json!([]))))
        .expect(0) // Must NOT be called when token is None.
        .mount(&server)
        .await;

    // Fallback mock (no auth requirement).
    Mock::given(method("GET"))
        .and(path("/admin/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(projects_body(json!([]))))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_projects(&uri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "list_projects(None) must succeed via the fallback mock, got: {:?}",
        result
    );
    // The `expect(0)` on the bearer-gated mock is verified at drop.
}

// ---------------------------------------------------------------------------
// Network failure — connection refused maps to Diagnostic::Network (AC 7)
// ---------------------------------------------------------------------------

/// A connection failure (nothing listening) must map to `Diagnostic::Network`,
/// not `ServerError`. Port 1 is reserved — the OS refuses the connection
/// immediately. Do NOT use a dropped `MockServer`: wiremock keeps its socket
/// alive through drop and 404s unmatched requests, which would yield
/// `ServerError`, not `Network`.
#[tokio::test]
async fn list_projects_connection_refused_returns_network() {
    let uri = "http://127.0.0.1:1".to_string();

    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_projects(&uri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err when server is unreachable");
    let err = result.unwrap_err();
    assert!(
        matches!(err, Diagnostic::Network(_)),
        "expected Diagnostic::Network for connection failure, got: {err:?}"
    );
}
