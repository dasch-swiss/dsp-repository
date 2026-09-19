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
// Project lookup endpoints are public (no bearer token required).
// Tests in this file assert:
//   1. The correct URL path is requested (shortcode / shortname / IRI variants).
//   2. No `Authorization` header is sent.
//   3. The response is correctly mapped to `ProjectRef`.
//   4. 404 maps to `Diagnostic::NotFound`.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Minimal DSP-API project response body.
fn project_body(id: &str, shortcode: &str, shortname: &str) -> serde_json::Value {
    json!({
        "project": {
            "id": id,
            "shortcode": shortcode,
            "shortname": shortname
        }
    })
}

// ---------------------------------------------------------------------------
// By-shortcode lookup
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_by_shortcode_requests_correct_path() {
    let server = MockServer::start().await;

    let mock = Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/0001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(project_body(
            "http://rdfh.ch/projects/0001",
            "0001",
            "anything",
        )))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resolve_project(&uri, "0001")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let project = result.unwrap();
    assert_eq!(project.iri, "http://rdfh.ch/projects/0001");
    assert_eq!(project.shortcode, "0001");
    assert_eq!(project.shortname, "anything");

    drop(mock);
}

#[tokio::test]
async fn resolve_by_shortcode_sends_no_authorization_header() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/0001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(project_body(
            "http://rdfh.ch/projects/0001",
            "0001",
            "anything",
        )))
        .mount(&server)
        .await;

    let uri = server.uri();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resolve_project(&uri, "0001")
    })
    .join()
    .expect("blocking thread should not panic");

    // Inspect received requests and assert no Authorization header was sent.
    let received = server
        .received_requests()
        .await
        .expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "expected exactly one request");
    assert!(
        !received[0].headers.contains_key("authorization"),
        "project lookup must NOT send an Authorization header (public endpoint)"
    );
}

// ---------------------------------------------------------------------------
// By-shortname lookup
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_by_shortname_requests_correct_path() {
    let server = MockServer::start().await;

    let mock = Mock::given(method("GET"))
        .and(path("/admin/projects/shortname/incunabula"))
        .respond_with(ResponseTemplate::new(200).set_body_json(project_body(
            "http://rdfh.ch/projects/0803",
            "0803",
            "incunabula",
        )))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resolve_project(&uri, "incunabula")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let project = result.unwrap();
    assert_eq!(project.iri, "http://rdfh.ch/projects/0803");
    assert_eq!(project.shortcode, "0803");
    assert_eq!(project.shortname, "incunabula");

    drop(mock);
}

// ---------------------------------------------------------------------------
// By-IRI lookup (percent-encoding)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_by_iri_encodes_path_segment_correctly() {
    let server = MockServer::start().await;

    // The IRI `http://rdfh.ch/projects/0001` must be percent-encoded as
    // `http%3A%2F%2Frdfh%2Ech%2Fprojects%2F0001` when NON_ALPHANUMERIC is used.
    // The path registered with wiremock must match this encoded form exactly.
    let expected_path = "/admin/projects/iri/http%3A%2F%2Frdfh%2Ech%2Fprojects%2F0001";

    let mock = Mock::given(method("GET"))
        .and(path(expected_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(project_body(
            "http://rdfh.ch/projects/0001",
            "0001",
            "anything",
        )))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resolve_project(&uri, "http://rdfh.ch/projects/0001")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);

    drop(mock);
}

#[tokio::test]
async fn resolve_by_iri_sends_no_authorization_header() {
    let server = MockServer::start().await;

    // Match any GET on the iri path prefix — the exact encoding is tested in
    // `resolve_by_iri_encodes_path_segment_correctly`.
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(project_body(
            "http://rdfh.ch/projects/0001",
            "0001",
            "anything",
        )))
        .mount(&server)
        .await;

    let uri = server.uri();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resolve_project(&uri, "http://rdfh.ch/projects/0001")
    })
    .join()
    .expect("blocking thread should not panic");

    let received = server
        .received_requests()
        .await
        .expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "expected exactly one request");
    assert!(
        !received[0].headers.contains_key("authorization"),
        "IRI-based project lookup must NOT send an Authorization header (public endpoint)"
    );
}

// ---------------------------------------------------------------------------
// 404 → NotFound
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_404_returns_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/dead"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resolve_project(&uri, "dead")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 404 response");
    let err = result.unwrap_err();
    assert!(
        matches!(err, Diagnostic::NotFound(_)),
        "expected Diagnostic::NotFound for 404, got: {:?}",
        err
    );

    // The NotFound message should reference the input project identifier.
    if let Diagnostic::NotFound(msg) = &err {
        assert!(
            msg.contains("dead"),
            "NotFound message should include the input identifier 'dead', got: {msg}"
        );
    }
}

#[tokio::test]
async fn resolve_404_by_iri_message_contains_project_id() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resolve_project(&uri, "http://rdfh.ch/projects/9999")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 404 response");
    let err = result.unwrap_err();
    assert!(
        matches!(err, Diagnostic::NotFound(_)),
        "expected Diagnostic::NotFound, got: {:?}",
        err
    );

    if let Diagnostic::NotFound(msg) = &err {
        assert!(
            msg.contains("http://rdfh.ch/projects/9999"),
            "NotFound message should include the IRI, got: {msg}"
        );
    }
}

// ---------------------------------------------------------------------------
// 5xx → ServerError
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resolve_500_returns_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resolve_project(&uri, "0001")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 500 response");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "expected Diagnostic::ServerError for 500 response"
    );
}
