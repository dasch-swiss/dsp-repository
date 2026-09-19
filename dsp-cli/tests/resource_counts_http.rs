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
// `resource_counts` (`GET /v3/projects/{enc(project_iri)}/resourcesPerOntology`)
// tests assert:
//   1. The correct URL path is requested (percent-encoded project IRI).
//   2. The response — a top-level JSON ARRAY of per-ontology entries — is flattened into a single
//      `HashMap<resource_class_iri, item_count>` across ALL ontology entries in the payload.
//   3. An ontology entry with an empty `classesAndCount` contributes no entries and does not error
//      (`#[serde(default)]`).
//   4. `Authorization: Bearer <token>` is sent when `token` is `Some`, and omitted entirely when
//      `token` is `None` (public endpoint, mirrors `list_data_models`).
//   5. Status-code mapping: 404 → `NotFound`; 401/403 → `AuthRequired` (generic
//      `map_unexpected_status`, not a bespoke message); other unexpected statuses (e.g. 500) →
//      `ServerError`.

use std::collections::HashMap;

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "test-token";
const PROJECT_IRI: &str = "http://rdfh.ch/projects/0001";
// URL-encoded form of PROJECT_IRI with NON_ALPHANUMERIC (same set used by enc()).
// Matches: http%3A%2F%2Frdfh%2Ech%2Fprojects%2F0001
const ENCODED_IRI: &str = "http%3A%2F%2Frdfh%2Ech%2Fprojects%2F0001";

fn expected_path() -> String {
    format!("/v3/projects/{ENCODED_IRI}/resourcesPerOntology")
}

// ---------------------------------------------------------------------------
// Happy path — multi-ontology, multi-class flattening
// ---------------------------------------------------------------------------

/// Two ontology entries, each with multiple classes. Asserts the returned map
/// flattens all entries' `classesAndCount` into one `{iri: count}` map with
/// the EXACT expected key/value pairs (not just `.len()`), since a bug that
/// only kept the last ontology's classes, or overwrote counts across
/// ontologies, would still pass a length-only check.
#[tokio::test]
async fn happy_path_flattens_multiple_ontologies_and_classes() {
    let server = MockServer::start().await;

    let body = json!([
        {
            "ontology": { "iri": "http://api.dasch.swiss/ontology/0001/onto-a/v2", "label": "Onto A" },
            "classesAndCount": [
                { "resourceClass": { "iri": "http://api.dasch.swiss/ontology/0001/onto-a/v2#Book" }, "itemCount": 42 },
                { "resourceClass": { "iri": "http://api.dasch.swiss/ontology/0001/onto-a/v2#Page" }, "itemCount": 1893 }
            ]
        },
        {
            "ontology": { "iri": "http://api.dasch.swiss/ontology/0001/onto-b/v2", "label": "Onto B" },
            "classesAndCount": [
                { "resourceClass": { "iri": "http://api.dasch.swiss/ontology/0001/onto-b/v2#Letter" }, "itemCount": 7 }
            ]
        }
    ]);

    Mock::given(method("GET"))
        .and(path(expected_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resource_counts(&uri, PROJECT_IRI, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let counts = result.unwrap();

    let expected: HashMap<String, u64> = HashMap::from([
        ("http://api.dasch.swiss/ontology/0001/onto-a/v2#Book".to_string(), 42),
        ("http://api.dasch.swiss/ontology/0001/onto-a/v2#Page".to_string(), 1893),
        ("http://api.dasch.swiss/ontology/0001/onto-b/v2#Letter".to_string(), 7),
    ]);

    assert_eq!(
        counts, expected,
        "flattened map must contain exactly these {{iri: count}} entries across BOTH ontologies"
    );
}

// ---------------------------------------------------------------------------
// Empty classesAndCount for one ontology entry
// ---------------------------------------------------------------------------

/// An ontology entry with `"classesAndCount": []` must contribute no map
/// entries and must not error — exercises `#[serde(default)]` on
/// `classes_and_count`.
#[tokio::test]
async fn empty_classes_and_count_contributes_no_entries_and_does_not_error() {
    let server = MockServer::start().await;

    let body = json!([
        {
            "ontology": { "iri": "http://api.dasch.swiss/ontology/0001/onto-empty/v2" },
            "classesAndCount": []
        },
        {
            "ontology": { "iri": "http://api.dasch.swiss/ontology/0001/onto-a/v2" },
            "classesAndCount": [
                { "resourceClass": { "iri": "http://api.dasch.swiss/ontology/0001/onto-a/v2#Book" }, "itemCount": 3 }
            ]
        }
    ]);

    Mock::given(method("GET"))
        .and(path(expected_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resource_counts(&uri, PROJECT_IRI, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let counts = result.unwrap();
    assert_eq!(counts.len(), 1, "empty classesAndCount entry must contribute zero map entries");
    assert_eq!(
        counts.get("http://api.dasch.swiss/ontology/0001/onto-a/v2#Book"),
        Some(&3),
        "the non-empty ontology entry's class must still be present"
    );
}

// ---------------------------------------------------------------------------
// Bearer header — present when token is Some
// ---------------------------------------------------------------------------

/// When `token = Some(TOKEN)` is passed, the request carries an
/// `Authorization: Bearer <token>` header.
#[tokio::test]
async fn bearer_header_sent_when_token_is_some() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(expected_path()))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resource_counts(&uri, PROJECT_IRI, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok with token, got: {:?}", result);

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    let auth = received[0]
        .headers
        .get("authorization")
        .expect("Authorization header must be present when token is Some");
    assert_eq!(
        auth.to_str().expect("header should be valid UTF-8"),
        format!("Bearer {TOKEN}"),
        "bearer token must match the value passed to resource_counts"
    );
}

// ---------------------------------------------------------------------------
// Bearer header — absent when token is None
// ---------------------------------------------------------------------------

/// When `token = None` is passed, NO `Authorization` header must be sent.
/// Mirrors `list_data_models`/`describe_data_model`'s public-endpoint pattern
/// (see `tests/data_model_list_http.rs::bearer_absent_when_token_is_none`).
#[tokio::test]
async fn bearer_header_absent_when_token_is_none() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(expected_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resource_counts(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok without token, got: {:?}", result);

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    assert!(
        received[0].headers.get("authorization").is_none(),
        "Authorization header must be ABSENT when token is None; \
         a future refactor must not start sending an unconditional bearer"
    );
}

// ---------------------------------------------------------------------------
// URL path — percent-encoded project IRI
// ---------------------------------------------------------------------------

/// Asserts the exact requested path includes the percent-encoded project IRI.
/// A regression that drops `enc()` in the impl would send a raw IRI
/// (containing ':', '/', '.') — this assertion catches it.
#[tokio::test]
async fn url_path_is_correctly_percent_encoded() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(expected_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resource_counts(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    let request_path = received[0].url.path();
    assert_eq!(
        request_path,
        expected_path(),
        "request path must be the percent-encoded IRI; \
         if this fails the impl may have dropped enc() encoding; \
         got: {request_path:?}"
    );
}

// ---------------------------------------------------------------------------
// Status-code mapping
// ---------------------------------------------------------------------------

#[tokio::test]
async fn status_404_returns_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(expected_path()))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resource_counts(&uri, PROJECT_IRI, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 404");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::NotFound(_)),
        "404 must map to Diagnostic::NotFound"
    );
}

#[tokio::test]
async fn status_401_returns_auth_required() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(expected_path()))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resource_counts(&uri, PROJECT_IRI, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 401");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::AuthRequired(_)),
        "401 must map to Diagnostic::AuthRequired via the shared map_unexpected_status"
    );
}

#[tokio::test]
async fn status_403_returns_auth_required() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(expected_path()))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resource_counts(&uri, PROJECT_IRI, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 403");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::AuthRequired(_)),
        "403 must map to Diagnostic::AuthRequired via the shared map_unexpected_status"
    );
}

#[tokio::test]
async fn status_500_returns_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(expected_path()))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.resource_counts(&uri, PROJECT_IRI, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 500");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "500 must map to Diagnostic::ServerError via the shared map_unexpected_status"
    );
}
