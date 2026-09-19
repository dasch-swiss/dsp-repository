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
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Produce a minimal JWT with the given JSON payload.
/// The secret is arbitrary — `extract_exp` disables signature validation.
fn make_jwt(payload: &serde_json::Value) -> String {
    encode(
        &Header::new(Algorithm::HS256),
        payload,
        &EncodingKey::from_secret(b"unused"),
    )
    .expect("test JWT encoding should not fail")
}

/// A JWT with an `exp` claim set to Unix timestamp 4_000_000_000 (year 2096
/// — safely in the future). Used by the 200 happy-path test to verify that
/// `LoginResponse.expires_at` is `Some`.
fn test_jwt_with_exp() -> String {
    let exp: i64 = 4_000_000_000;
    make_jwt(&json!({ "exp": exp }))
}

#[tokio::test]
async fn login_200_happy_path() {
    let server = MockServer::start().await;
    let token = test_jwt_with_exp();
    let token_clone = token.clone();

    let mock = Mock::given(method("POST"))
        .and(path("/v2/authentication"))
        .and(body_partial_json(json!({
            "email": "user@example.com",
            "password": "hunter2"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "token": token })))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.login(&uri, "user@example.com", "hunter2")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let response = result.unwrap();
    assert_eq!(
        response.token, token_clone,
        "token should match mock response"
    );
    assert_eq!(
        response.user, "user@example.com",
        "user should be echoed from input"
    );
    assert!(
        response.expires_at.is_some(),
        "expires_at should be Some because JWT carries exp claim"
    );

    // `mock` drop verifies the `.expect(1)` assertion automatically.
    drop(mock);
}

#[tokio::test]
async fn login_401_returns_auth_required_without_username_leak() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/authentication"))
        .and(body_partial_json(json!({
            "email": "user@example.com",
            "password": "hunter2"
        })))
        .respond_with(ResponseTemplate::new(401).set_body_string("bad credentials"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.login(&uri, "user@example.com", "hunter2")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 401 response");
    let err = result.unwrap_err();

    assert!(
        matches!(err, Diagnostic::AuthRequired(_)),
        "expected Diagnostic::AuthRequired, got: {:?}",
        err
    );

    // ADR-0007 / PRD AC 7: username MUST NOT appear in the error message.
    if let Diagnostic::AuthRequired(msg) = &err {
        assert!(
            !msg.contains("user@example.com"),
            "error message must not leak the username (ADR-0007 regression guard), got: {msg}"
        );
    }
}

#[tokio::test]
async fn login_500_returns_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/authentication"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.login(&uri, "user@example.com", "hunter2")
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
async fn login_403_returns_auth_required_without_username_leak() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/authentication"))
        .and(body_partial_json(json!({
            "email": "user@example.com",
            "password": "hunter2"
        })))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.login(&uri, "user@example.com", "hunter2")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 403 response");
    let err = result.unwrap_err();

    assert!(
        matches!(err, Diagnostic::AuthRequired(_)),
        "expected Diagnostic::AuthRequired for 403 response, got: {:?}",
        err
    );

    // ADR-0007 / PRD AC 7: username MUST NOT appear in the error message.
    if let Diagnostic::AuthRequired(msg) = &err {
        assert!(
            !msg.contains("user@example.com"),
            "error message must not leak the username (ADR-0007 regression guard), got: {msg}"
        );
    }
}

#[tokio::test]
async fn login_unparseable_2xx_body_returns_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/authentication"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>500</html>"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.login(&uri, "user@example.com", "hunter2")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 200 with malformed body");
    let err = result.unwrap_err();

    // PRD AC 10: a malformed 2xx body must map to ServerError, not Internal.
    assert!(
        matches!(err, Diagnostic::ServerError(_)),
        "expected Diagnostic::ServerError for unparseable 2xx body (PRD AC 10), got: {:?}",
        err
    );
}

#[tokio::test]
async fn login_200_with_username_sends_username_key() {
    let server = MockServer::start().await;
    let token = test_jwt_with_exp();
    let token_clone = token.clone();

    let mock = Mock::given(method("POST"))
        .and(path("/v2/authentication"))
        .and(body_partial_json(json!({
            "username": "jdoe",
            "password": "hunter2"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "token": token })))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.login(&uri, "jdoe", "hunter2")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "expected Ok for username login, got: {:?}",
        result
    );
    let response = result.unwrap();
    assert_eq!(
        response.token, token_clone,
        "token should match mock response"
    );
    assert_eq!(response.user, "jdoe", "user should be echoed from input");

    drop(mock);
}

#[tokio::test]
async fn login_200_with_iri_sends_iri_key() {
    let server = MockServer::start().await;
    let token = test_jwt_with_exp();
    let token_clone = token.clone();

    let mock = Mock::given(method("POST"))
        .and(path("/v2/authentication"))
        .and(body_partial_json(json!({
            "iri": "http://rdfh.ch/users/jane",
            "password": "hunter2"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "token": token })))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.login(&uri, "http://rdfh.ch/users/jane", "hunter2")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "expected Ok for IRI login, got: {:?}",
        result
    );
    let response = result.unwrap();
    assert_eq!(
        response.token, token_clone,
        "token should match mock response"
    );
    assert_eq!(
        response.user, "http://rdfh.ch/users/jane",
        "user should be echoed from input"
    );

    drop(mock);
}

#[tokio::test]
async fn login_401_with_username_returns_auth_required_without_identifier_leak() {
    // The non-disclosure guarantee (ADR-0007 / PRD AC 7) must hold for every
    // identifier type, not just email — the error message is identifier-agnostic.
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/authentication"))
        .and(body_partial_json(json!({
            "username": "jdoe",
            "password": "hunter2"
        })))
        .respond_with(ResponseTemplate::new(401).set_body_string("bad credentials"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.login(&uri, "jdoe", "hunter2")
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 401 response");
    let err = result.unwrap_err();

    assert!(
        matches!(err, Diagnostic::AuthRequired(_)),
        "expected Diagnostic::AuthRequired, got: {:?}",
        err
    );

    // ADR-0007 / PRD AC 7: the identifier MUST NOT appear in the error message.
    if let Diagnostic::AuthRequired(msg) = &err {
        assert!(
            !msg.contains("jdoe"),
            "error message must not leak the username (ADR-0007 regression guard), got: {msg}"
        );
    }
}

#[tokio::test]
async fn login_network_failure_returns_network_diagnostic() {
    // Use a URL that will result in a connection-refused error. Port 1 is
    // reserved and will not have anything listening, so the OS-level connect
    // will fail immediately. We do not rely on dropping a MockServer because
    // wiremock keeps the socket alive through the drop — the server returns 404
    // for unmatched requests rather than refusing the connection, which would
    // yield Diagnostic::NotFound instead of Diagnostic::Network.
    let uri = "http://127.0.0.1:1".to_string();

    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.login(&uri, "user@example.com", "hunter2")
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
