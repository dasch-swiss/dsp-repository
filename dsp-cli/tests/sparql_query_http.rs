// HTTP integration tests for `DspClient::sparql_query` (plan 035) against
// `HttpDspClient`.
//
// `reqwest::blocking` is safe alongside `#[tokio::test]` (which wiremock
// requires) provided the blocking client is constructed, used, and dropped
// entirely on a plain OS thread — not on the tokio runtime's thread pool.
// We achieve this with `std::thread::spawn` + `JoinHandle::join`: the
// blocking reqwest runtime lives and dies on its own OS thread, so it never
// tries to drop a Tokio runtime from within an async context (which would
// panic). Do not "fix" this by moving the `HttpDspClient::new()` call back
// into the async body without also dropping the blocking runtime on a
// non-async thread. Mirrors `tests/vocabulary_http.rs`.
//
// The endpoint is never anonymous (D11): every call sends
// `Authorization: Bearer <token>`. Use a clearly synthetic token — never a
// real JWT — so a failed assertion can never print a real credential.

use std::time::Duration;

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use serde_json::json;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "test-token";
const QUERY: &str = "SELECT * WHERE { ?s ?p ?o } LIMIT 10";

// ---------------------------------------------------------------------------
// 2xx relay
// ---------------------------------------------------------------------------

/// A 2xx response is relayed byte-exact, with its `Content-Type` surfaced.
#[tokio::test]
async fn success_relays_body_and_content_type() {
    let server = MockServer::start().await;

    let body = json!({"head": {"vars": ["s", "p", "o"]}, "results": {"bindings": []}});
    let body_bytes = serde_json::to_vec(&body).expect("serialize fixture");

    Mock::given(method("POST"))
        .and(path("/admin/sparql/query"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/sparql-results+json")
                .set_body_bytes(body_bytes.clone()),
        )
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.sparql_query(&uri, TOKEN, QUERY, "application/sparql-results+json", 3600)
    })
    .join()
    .expect("blocking thread should not panic");

    let resp = result.expect("expected Ok for a 2xx relay");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.content_type.as_deref(), Some("application/sparql-results+json"));
    assert_eq!(resp.body, body_bytes, "body must be byte-exact");
}

// ---------------------------------------------------------------------------
// Outgoing request shape
// ---------------------------------------------------------------------------

/// The outgoing request must carry `Content-Type: application/sparql-query`,
/// `Authorization: Bearer <token>`, `Accept` exactly as passed, the raw query
/// as the body, and no query string on the URL (D6).
#[tokio::test]
async fn outgoing_request_shape_is_correct() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/admin/sparql/query"))
        .and(header("content-type", "application/sparql-query"))
        .and(header("authorization", format!("Bearer {TOKEN}")))
        .and(header("accept", "text/csv"))
        .and(body_string(QUERY))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/csv")
                .set_body_string("s,p,o\n"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.sparql_query(&uri, TOKEN, QUERY, "text/csv", 3600)
    })
    .join()
    .expect("blocking thread should not panic");

    result.expect("expected Ok — the mock above only matches the exact shape asserted");

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    assert!(
        received[0].url.query().is_none(),
        "the URL must carry no query string (D6); got: {}",
        received[0].url
    );
}

// ---------------------------------------------------------------------------
// Store non-2xx is relayed, not an Err
// ---------------------------------------------------------------------------

/// A store `400` (malformed query, D8's note) is relayed as `Ok(SparqlResponse)`,
/// not `Err` — the action, not the client, decides what a non-2xx relay means.
#[tokio::test]
async fn store_400_is_relayed_as_ok_not_err() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/admin/sparql/query"))
        .respond_with(
            ResponseTemplate::new(400)
                .insert_header("Content-Type", "text/plain")
                .set_body_string("Parse error: line 1, column 1: encountered nonsense"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.sparql_query(&uri, TOKEN, "not a query", "application/sparql-results+json", 3600)
    })
    .join()
    .expect("blocking thread should not panic");

    let resp = result.expect("a store 400 must be Ok(SparqlResponse), not Err");
    assert_eq!(resp.status, 400);
    assert_eq!(resp.content_type.as_deref(), Some("text/plain"));
    assert!(resp.body.starts_with(b"Parse error"));
}

/// An arbitrary unlisted status (not in D8's table) proves the `Relay`
/// default is pinned by a test, not just by prose.
#[tokio::test]
async fn unlisted_status_418_relays_as_ok() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/admin/sparql/query"))
        .respond_with(ResponseTemplate::new(418).set_body_string("I'm a teapot"))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.sparql_query(&uri, TOKEN, QUERY, "application/sparql-results+json", 3600)
    })
    .join()
    .expect("blocking thread should not panic");

    let resp = result.expect("an unlisted status must relay as Ok, per the Relay default");
    assert_eq!(resp.status, 418);
}

// ---------------------------------------------------------------------------
// dsp-api's own typed failures (D8's table)
// ---------------------------------------------------------------------------

async fn mount_status(server: &MockServer, status: u16, content_type: Option<&str>, body: &str) {
    let mut template = ResponseTemplate::new(status);
    if let Some(ct) = content_type {
        template = template.insert_header("Content-Type", ct);
    }
    template = template.set_body_string(body);
    Mock::given(method("POST"))
        .and(path("/admin/sparql/query"))
        .respond_with(template)
        .expect(1)
        .mount(server)
        .await;
}

fn run_query(uri: String) -> Result<dsp_cli::client::sparql::SparqlResponse, Diagnostic> {
    run_query_with_timeout(uri, 3600)
}

fn run_query_with_timeout(
    uri: String,
    timeout_secs: u64,
) -> Result<dsp_cli::client::sparql::SparqlResponse, Diagnostic> {
    std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.sparql_query(&uri, TOKEN, QUERY, "application/sparql-results+json", timeout_secs)
    })
    .join()
    .expect("blocking thread should not panic")
}

#[tokio::test]
async fn status_401_maps_to_auth_required() {
    let server = MockServer::start().await;
    mount_status(&server, 401, None, "").await;

    let err = run_query(server.uri()).expect_err("401 must be Err");
    match err {
        Diagnostic::AuthRequired(msg) => {
            assert!(msg.contains("login"), "message should point at re-login: {msg}");
        }
        other => panic!("expected AuthRequired, got: {other:?}"),
    }
}

/// D8: a 403 message must say re-login will NOT help.
#[tokio::test]
async fn status_403_maps_to_auth_required_with_no_relogin_wording() {
    let server = MockServer::start().await;
    mount_status(&server, 403, None, "").await;

    let err = run_query(server.uri()).expect_err("403 must be Err");
    match err {
        Diagnostic::AuthRequired(msg) => {
            assert!(
                msg.contains("system administrator"),
                "message must explain the SystemAdmin requirement: {msg}"
            );
            // Assert the POSITIVE property D8 cares about. A negative
            // assertion on one phrasing would also pass for "please log in
            // again", which is exactly the regression that matters: dsp-cli/ADR-0012
            // makes exit 3 the signal an agent may act on by running
            // `dsp auth login`, so a 403 must say outright that it won't help.
            assert!(
                msg.contains("will not help"),
                "403 message must state that re-login will not help: {msg}"
            );
        }
        other => panic!("expected AuthRequired, got: {other:?}"),
    }
}

/// D9: the 404 message must name the server.
#[tokio::test]
async fn status_404_maps_to_not_found_naming_the_server() {
    let server = MockServer::start().await;
    mount_status(&server, 404, None, "").await;

    let uri = server.uri();
    let err = run_query(uri.clone()).expect_err("404 must be Err");
    match err {
        Diagnostic::NotFound(msg) => {
            assert!(msg.contains(&uri), "404 message must name the server it applies to; got: {msg}");
            assert!(
                msg.contains("allow-sparql-passthrough"),
                "404 message must mention the guardrail flag: {msg}"
            );
        }
        other => panic!("expected NotFound, got: {other:?}"),
    }
}

#[tokio::test]
async fn status_413_maps_to_usage() {
    let server = MockServer::start().await;
    mount_status(&server, 413, None, "").await;

    let err = run_query(server.uri()).expect_err("413 must be Err");
    assert!(matches!(err, Diagnostic::Usage(_)), "expected Usage, got: {err:?}");
}

#[tokio::test]
async fn status_415_maps_to_internal() {
    let server = MockServer::start().await;
    mount_status(&server, 415, None, "").await;

    let err = run_query(server.uri()).expect_err("415 must be Err");
    assert!(matches!(err, Diagnostic::Internal(_)), "expected Internal, got: {err:?}");
}

#[tokio::test]
async fn status_500_with_parseable_message_uses_it() {
    let server = MockServer::start().await;
    mount_status(
        &server,
        500,
        Some("application/json"),
        r#"{"message": "The response exceeds the configured limit of 64 MiB."}"#,
    )
    .await;

    let err = run_query(server.uri()).expect_err("500 must be Err");
    match err {
        Diagnostic::ServerError(msg) => {
            assert!(
                msg.contains("exceeds the configured limit"),
                "expected the server's parsed message, got: {msg}"
            );
        }
        other => panic!("expected ServerError, got: {other:?}"),
    }
}

/// The 5xx arm must **always name the status**, and an empty body must not
/// yield a bare `Error: server error:` with no cause. Both behaviours were
/// introduced by the round-1 review fix and were unpinned until now.
#[tokio::test]
async fn status_503_with_empty_body_still_names_the_status() {
    let server = MockServer::start().await;
    mount_status(&server, 503, None, "").await;

    let err = run_query(server.uri()).expect_err("503 must be Err");
    match err {
        Diagnostic::ServerError(msg) => {
            assert!(
                msg.contains("HTTP 503"),
                "an empty 5xx body must still name the status: {msg:?}"
            );
            assert!(!msg.trim().is_empty(), "message must not be empty: {msg:?}");
        }
        other => panic!("expected ServerError, got: {other:?}"),
    }
}

/// A non-JSON 5xx body is reported with its status too, not as bare store text.
#[tokio::test]
async fn status_500_non_json_body_names_the_status() {
    let server = MockServer::start().await;
    mount_status(&server, 500, Some("text/plain"), "the store exploded").await;

    let err = run_query(server.uri()).expect_err("500 must be Err");
    match err {
        Diagnostic::ServerError(msg) => {
            assert!(msg.contains("HTTP 500"), "must name the status: {msg:?}");
            assert!(msg.contains("the store exploded"), "must carry the body text: {msg:?}");
        }
        other => panic!("expected ServerError, got: {other:?}"),
    }
}

/// Redirects are deliberately not followed (a 307/308 would replay the request
/// body — the query text — to a third host). A 3xx therefore reaches the caller
/// as a relay rather than being chased.
#[tokio::test]
async fn redirects_are_not_followed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/admin/sparql/query"))
        .respond_with(ResponseTemplate::new(307).insert_header("Location", "http://127.0.0.1:1/elsewhere"))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.sparql_query(&uri, TOKEN, QUERY, "application/sparql-results+json", 3600)
    })
    .join()
    .expect("blocking thread should not panic");

    let resp = result.expect("a 3xx must relay, not be followed or error");
    assert_eq!(resp.status, 307, "the redirect itself must be surfaced");
}

/// D7's invariant has **no carve-out for a body that happens to parse.** The
/// `application/json` + `message` gate proves only the shape, not the author:
/// the store, a proxy, or a hostile `--server` can all produce it, and the
/// string is printed as prose to a terminal. So the *parsed* message goes
/// through sanitise-and-cap too — control characters stripped, length capped.
///
/// This case was unpinned when the feature first landed and the parsed path was
/// genuinely unsanitised (found in review, 2026-08-07). Keep it.
#[tokio::test]
async fn status_500_parsed_message_is_sanitised_and_capped() {
    let server = MockServer::start().await;
    // An ANSI escape that would retitle a terminal window, plus a body far
    // longer than the 200-char cap.
    let hostile = format!("\u{1b}]0;pwned\u{7}{}", "A".repeat(5000));
    mount_status(
        &server,
        500,
        Some("application/json"),
        &serde_json::to_string(&serde_json::json!({ "message": hostile })).expect("json"),
    )
    .await;

    let err = run_query(server.uri()).expect_err("500 must be Err");
    match err {
        Diagnostic::ServerError(msg) => {
            assert!(!msg.contains('\u{1b}'), "ESC must not survive into a prose diagnostic: {msg:?}");
            assert!(!msg.contains('\u{7}'), "BEL must not survive into a prose diagnostic: {msg:?}");
            assert!(
                msg.chars().count() < 300,
                "message must be capped, got {} chars",
                msg.chars().count()
            );
        }
        other => panic!("expected ServerError, got: {other:?}"),
    }
}

/// A 500 without a parseable `{"message"}` body falls back to the raw body,
/// through the shared sanitise-and-cap helper — capped, not leaked whole.
#[tokio::test]
async fn status_500_without_parseable_message_falls_back_capped() {
    let server = MockServer::start().await;
    let long_body = "x".repeat(500);
    mount_status(&server, 500, Some("text/plain"), &long_body).await;

    let err = run_query(server.uri()).expect_err("500 must be Err");
    match err {
        Diagnostic::ServerError(msg) => {
            assert!(msg.len() < 500, "message must be capped, got len {}", msg.len());
            assert!(msg.ends_with('…'), "capped message must end with an ellipsis: {msg}");
        }
        other => panic!("expected ServerError, got: {other:?}"),
    }
}

#[tokio::test]
async fn status_502_maps_to_server_error() {
    let server = MockServer::start().await;
    mount_status(
        &server,
        502,
        Some("application/json"),
        r#"{"message": "The API could not authenticate with the triplestore."}"#,
    )
    .await;

    let err = run_query(server.uri()).expect_err("502 must be Err");
    assert!(matches!(err, Diagnostic::ServerError(_)), "expected ServerError, got: {err:?}");
}

#[tokio::test]
async fn status_503_maps_to_server_error() {
    let server = MockServer::start().await;
    mount_status(
        &server,
        503,
        Some("application/json"),
        r#"{"message": "The triplestore is unavailable."}"#,
    )
    .await;

    let err = run_query(server.uri()).expect_err("503 must be Err");
    assert!(matches!(err, Diagnostic::ServerError(_)), "expected ServerError, got: {err:?}");
}

#[tokio::test]
async fn status_504_maps_to_server_error() {
    let server = MockServer::start().await;
    mount_status(
        &server,
        504,
        Some("application/json"),
        r#"{"message": "The SPARQL passthrough request exceeded the configured time limit of 120 seconds."}"#,
    )
    .await;

    let err = run_query(server.uri()).expect_err("504 must be Err");
    assert!(matches!(err, Diagnostic::ServerError(_)), "expected ServerError, got: {err:?}");
}

// ---------------------------------------------------------------------------
// Transport failure (D17)
// ---------------------------------------------------------------------------

/// A transport-level failure maps to `Diagnostic::Network`, with wording that
/// distinguishes a client-side failure from the server's own `504`.
///
/// Port 1 is reserved and will not have anything listening, so the OS-level
/// connect fails immediately — a real transport failure, not any HTTP
/// status. We do not rely on dropping a `MockServer` (as tried initially):
/// wiremock's socket can outlive `drop()` long enough that a request either
/// hits a *different* still-running mock server that happens to have been
/// assigned the same ephemeral port (test-parallelism port reuse), or gets a
/// `404` from wiremock's own "unmatched request" handler — either way,
/// `Diagnostic::NotFound` rather than `Diagnostic::Network`. `login_http.rs`'s
/// `login_network_failure_returns_network_diagnostic` documents the same
/// finding; mirrored here.
#[tokio::test]
async fn transport_failure_maps_to_network_not_a_status() {
    let uri = "http://127.0.0.1:1".to_string();

    let err = run_query(uri).expect_err("a transport failure must be Err");
    match err {
        Diagnostic::Network(msg) => {
            // Must not be phrased as if it were the server's own 504.
            assert!(
                !msg.contains("504"),
                "a client-side transport failure must not read like the server's 504: {msg}"
            );
        }
        other => panic!("expected Network, got: {other:?}"),
    }
}

/// A response delayed via `ResponseTemplate::set_delay`, but comfortably
/// under the timeout passed to `sparql_query`, still succeeds — the
/// non-timeout half of the D17 test-plan row's mechanism.
#[tokio::test]
async fn delayed_response_within_bound_still_succeeds() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/admin/sparql/query"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(50))
                .insert_header("Content-Type", "application/sparql-results+json")
                .set_body_string("{}"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let resp = run_query(server.uri()).expect("a bounded delay must still succeed");
    assert_eq!(resp.status, 200);
}

/// D17: `--timeout` reaches the client. Since the timeout is threaded through
/// `sparql_query`'s `timeout_secs` parameter and the client is built per call
/// (not once at `HttpDspClient::new()`), a short `timeout_secs` here really
/// does bound the request — unlike before this amendment, when the client's
/// 180s timeout was fixed at construction and unreachable from a test in
/// under three minutes. A response delayed past a 1s timeout must map to
/// `Diagnostic::Network`, worded distinguishably from the server's own `504`.
#[tokio::test]
async fn short_timeout_past_delay_maps_to_network() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/admin/sparql/query"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(2))
                .insert_header("Content-Type", "application/sparql-results+json")
                .set_body_string("{}"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let err = run_query_with_timeout(server.uri(), 1).expect_err("a delay past --timeout must be Err");
    match err {
        Diagnostic::Network(msg) => {
            // The message must name the client-side origin explicitly. It is
            // allowed — expected, even — to mention "504" while explaining
            // the *contrast* with the server's own guardrail; the property
            // under test is that it never reads as if a 504 was actually
            // received (i.e. it must not say the server responded/returned
            // one).
            assert!(
                msg.contains("timed out on the client side"),
                "a client-side timeout must name its own origin explicitly: {msg}"
            );
            assert!(
                !msg.contains("the server responded")
                    && !msg.contains("server returned")
                    && !msg.contains("got 504")
                    && !msg.contains("got HTTP 504"),
                "must not claim the server actually returned a 504: {msg}"
            );
        }
        other => panic!("expected Network, got: {other:?}"),
    }
}
