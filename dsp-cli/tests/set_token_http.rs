// `reqwest::blocking` is safe alongside `#[tokio::test]` (which wiremock
// requires) provided the blocking client is constructed, used, and dropped
// entirely on a plain OS thread — not on the tokio runtime's thread pool.
// We achieve this with `std::thread::spawn` + `JoinHandle::join`: the
// blocking reqwest runtime lives and dies on its own OS thread, so it never
// tries to drop a Tokio runtime from within an async context (which would
// panic). Do not "fix" this by moving the `HttpDspClient::new()` call back
// into the async body without also dropping the blocking runtime on a
// non-async thread.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A fixed test token that we pass to `verify_token`.
///
/// The value is arbitrary — `verify_token` sends it as a bearer token to the
/// server; the mock server checks only the `Authorization` header value.
const TEST_TOKEN: &str = "test-bearer-token-do-not-log";

#[tokio::test]
async fn verify_token_200_returns_ok() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v2/authentication"))
        .and(header("Authorization", format!("Bearer {TEST_TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&server)
        .await;

    let uri = server.uri();
    let token = TEST_TOKEN.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.verify_token(&uri, &token)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok for 200 response, got: {:?}", result);
}

#[tokio::test]
async fn verify_token_401_returns_auth_required_without_token_leak() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v2/authentication"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let token = TEST_TOKEN.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.verify_token(&uri, &token)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 401 response");
    let err = result.unwrap_err();

    assert!(
        matches!(err, Diagnostic::AuthRequired(_)),
        "expected Diagnostic::AuthRequired for 401, got: {:?}",
        err
    );

    // Token MUST NOT appear in the error message.
    assert!(
        !err.to_string().contains(TEST_TOKEN),
        "error message must not leak the token, got: {}",
        err
    );
}

#[tokio::test]
async fn verify_token_403_returns_auth_required_without_token_leak() {
    // 403 is a DISTINCT branch from 401 — both must map to AuthRequired.
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v2/authentication"))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let token = TEST_TOKEN.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.verify_token(&uri, &token)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 403 response");
    let err = result.unwrap_err();

    assert!(
        matches!(err, Diagnostic::AuthRequired(_)),
        "expected Diagnostic::AuthRequired for 403 (distinct branch from 401), got: {:?}",
        err
    );

    // Token MUST NOT appear in the error message.
    assert!(
        !err.to_string().contains(TEST_TOKEN),
        "error message must not leak the token, got: {}",
        err
    );
}

#[tokio::test]
async fn verify_token_404_returns_server_error() {
    // A 404 is a non-5xx unexpected status — must go through `map_unexpected_status`
    // and yield ServerError, not NotFound. The `verify_token` path has no special
    // 404 branch (unlike `login` or `resolve_project`).
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v2/authentication"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let token = TEST_TOKEN.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.verify_token(&uri, &token)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 404 response");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "expected Diagnostic::ServerError for 404 (non-5xx unexpected status)"
    );
}

#[tokio::test]
async fn verify_token_500_returns_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v2/authentication"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let token = TEST_TOKEN.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.verify_token(&uri, &token)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 500 response");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "expected Diagnostic::ServerError for 500 response"
    );
}

#[tokio::test]
async fn verify_token_connection_refused_returns_network() {
    // Use port 1 — reserved, nothing listens there, OS refuses the connection
    // immediately. Do NOT rely on dropping a MockServer to get connection
    // refused: wiremock keeps its socket alive through drop and returns 404
    // for unmatched requests, which would yield ServerError, not Network.
    let uri = "http://127.0.0.1:1".to_string();
    let token = TEST_TOKEN.to_string();

    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.verify_token(&uri, &token)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err when server is unreachable");
    let err = result.unwrap_err();
    assert!(
        matches!(err, Diagnostic::Network(_)),
        "expected Diagnostic::Network for connection failure, got: {:?}",
        err
    );
}
