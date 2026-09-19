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
// All fixture IRIs use the `api.dasch.swiss` namespace and match real shapes
// verified against the live API. The endpoint is public: an unauthenticated
// call (token = None) omits the Authorization header entirely; an
// authenticated call sends `Authorization: Bearer <token>`.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use serde_json::json;
use wiremock::matchers::{header, header_exists, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

// A clearly synthetic token — never a real JWT.
const TOKEN: &str = "test-token";

// The project IRI used across all tests (BEOL project on api.dasch.swiss).
const PROJECT_IRI: &str = "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build the expected URL path for the metadata endpoint (percent-encoded IRI).
fn metadata_path() -> String {
    // The IRI is percent-encoded by `enc()` in the impl. We encode it the same
    // way: every non-alphanumeric character is encoded with `%XX`.
    let encoded: String = PROJECT_IRI
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    format!("/v2/ontologies/metadata/{encoded}")
}

/// Mount a mock that returns `body` as a 200 JSON response.
///
/// Uses an exact `path(metadata_path())` matcher so a regression that drops
/// percent-encoding in the IRI path segment will cause the mock to receive zero
/// hits (wiremock's unmatched-request fallback returns 404, turning the test red).
async fn mount_200(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("GET"))
        .and(path(metadata_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

// ---------------------------------------------------------------------------
// Multi — @graph with 2 ontologies
// ---------------------------------------------------------------------------

/// `@graph` with 2 items: one missing `rdfs:label`, one missing
/// `knora-api:lastModificationDate`. Both must be `DataModel` values with
/// `is_builtin: false`; the present/absent fields map to `Some`/`None`.
#[tokio::test]
async fn multi_graph_returns_two_data_models() {
    let server = MockServer::start().await;

    let body = json!({
        "@graph": [
            {
                "@id": "http://api.dasch.swiss/ontology/0801/beol/v2",
                "rdfs:label": "The BEOL ontology",
                "knora-api:lastModificationDate": {
                    "@value": "2024-05-27T13:43:26.233048Z",
                    "@type": "xsd:dateTimeStamp"
                }
            },
            {
                "@id": "http://api.dasch.swiss/ontology/0801/biblio/v2"
                // rdfs:label absent → label: None
                // knora-api:lastModificationDate absent → last_modified: None
            }
        ],
        "@context": {}
    });

    mount_200(&server, body).await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_data_models(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let models = result.unwrap();
    assert_eq!(models.len(), 2, "expected 2 data models");

    // Model 1: beol — has both label and last_modified
    let m1 = models
        .iter()
        .find(|m| m.name == "beol")
        .expect("beol must be present");
    assert_eq!(m1.iri, "http://api.dasch.swiss/ontology/0801/beol/v2");
    assert_eq!(m1.label.as_deref(), Some("The BEOL ontology"));
    assert!(
        m1.last_modified.is_some(),
        "beol last_modified must be Some"
    );
    assert!(
        m1.last_modified
            .as_deref()
            .unwrap()
            .starts_with("2024-05-27"),
        "last_modified should contain the date"
    );
    assert!(!m1.is_builtin, "project data-model must not be builtin");

    // Model 2: biblio — missing both label and last_modified
    let m2 = models
        .iter()
        .find(|m| m.name == "biblio")
        .expect("biblio must be present");
    assert_eq!(m2.iri, "http://api.dasch.swiss/ontology/0801/biblio/v2");
    assert!(m2.label.is_none(), "biblio label must be None (absent)");
    assert!(
        m2.last_modified.is_none(),
        "biblio last_modified must be None (absent)"
    );
    assert!(!m2.is_builtin);
}

// ---------------------------------------------------------------------------
// Graph of length 1 — @graph wrapping a single ontology
// ---------------------------------------------------------------------------

/// `@graph` with exactly one item. This guards against the reconciliation
/// regressing: the `@graph` arm must be taken even for a length-1 array,
/// yielding exactly 1 `DataModel`.
#[tokio::test]
async fn graph_of_length_one_returns_one_data_model() {
    let server = MockServer::start().await;

    let body = json!({
        "@graph": [
            {
                "@id": "http://api.dasch.swiss/ontology/0862/gotthelf/v2",
                "rdfs:label": "gotthelf-ontology",
                "knora-api:lastModificationDate": {
                    "@value": "2023-12-01T10:00:00Z",
                    "@type": "xsd:dateTimeStamp"
                }
            }
        ],
        "@context": {}
    });

    mount_200(&server, body).await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_data_models(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let models = result.unwrap();
    assert_eq!(
        models.len(),
        1,
        "a length-1 @graph must yield exactly 1 DataModel"
    );
    assert_eq!(models[0].name, "gotthelf");
    assert_eq!(models[0].label.as_deref(), Some("gotthelf-ontology"));
    assert!(!models[0].is_builtin);
}

// ---------------------------------------------------------------------------
// Single flattened — top-level @id, no @graph
// ---------------------------------------------------------------------------

/// The critical JSON-LD edge: when there is exactly one ontology, the server
/// returns it flattened at the top level (no `@graph`). The reconciliation
/// must detect the top-level `@id` and build a single-element `Vec`.
#[tokio::test]
async fn single_flattened_returns_one_data_model() {
    let server = MockServer::start().await;

    // Verified shape from api.dasch.swiss project 0862 (one ontology).
    let body = json!({
        "@id": "http://api.dasch.swiss/ontology/0862/gotthelf/v2",
        "rdfs:label": "gotthelf-ontology",
        "knora-api:lastModificationDate": {
            "@value": "2023-12-01T10:00:00Z",
            "@type": "xsd:dateTimeStamp"
        },
        "@context": {}
    });

    mount_200(&server, body).await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_data_models(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let models = result.unwrap();
    assert_eq!(
        models.len(),
        1,
        "flattened single-ontology response must yield 1 DataModel"
    );
    assert_eq!(models[0].name, "gotthelf");
    assert_eq!(
        models[0].iri,
        "http://api.dasch.swiss/ontology/0862/gotthelf/v2"
    );
    assert_eq!(models[0].label.as_deref(), Some("gotthelf-ontology"));
    assert!(models[0].last_modified.is_some());
    assert!(!models[0].is_builtin);
}

// ---------------------------------------------------------------------------
// Zero — empty object {}
// ---------------------------------------------------------------------------

/// Verified shape from a project with no ontologies: the server returns `{}`.
/// Must produce an empty `Vec` (not an error).
#[tokio::test]
async fn zero_ontologies_returns_empty_vec() {
    let server = MockServer::start().await;

    mount_200(&server, json!({})).await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_data_models(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "empty object must yield Ok, got: {:?}",
        result
    );
    assert!(
        result.unwrap().is_empty(),
        "empty object must yield empty Vec"
    );
}

// ---------------------------------------------------------------------------
// Bearer present
// ---------------------------------------------------------------------------

/// When `token = Some("test-token")` is passed, the request carries an
/// `Authorization: Bearer test-token` header.
#[tokio::test]
async fn bearer_present_when_token_is_some() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(metadata_path()))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_data_models(&uri, PROJECT_IRI, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok with token, got: {:?}", result);

    let received = server
        .received_requests()
        .await
        .expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    let auth = received[0]
        .headers
        .get("authorization")
        .expect("Authorization header must be present when token is Some");
    assert_eq!(
        auth.to_str().expect("header should be valid UTF-8"),
        format!("Bearer {TOKEN}"),
        "bearer token must match the value passed to list_data_models"
    );
    // Assert the IRI was percent-encoded in the URL path. A regression that drops
    // `enc()` in the impl would send a raw IRI (containing ':', '/', '.') — this
    // assertion catches it.
    let request_path = received[0].url.path();
    assert_eq!(
        request_path,
        metadata_path(),
        "request path must be the percent-encoded IRI; \
         if this fails the impl may have dropped enc() encoding; \
         got: {request_path:?}"
    );
}

// ---------------------------------------------------------------------------
// Bearer absent
// ---------------------------------------------------------------------------

/// When `token = None` is passed, NO `Authorization` header must be sent.
/// Guards against a future refactor accidentally sending an unconditional bearer.
#[tokio::test]
async fn bearer_absent_when_token_is_none() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex("^/v2/ontologies/metadata/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_data_models(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    let received = server
        .received_requests()
        .await
        .expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    assert!(
        received[0].headers.get("authorization").is_none(),
        "Authorization header must be ABSENT when token is None; \
         a future refactor must not start sending an unconditional bearer"
    );
}

/// Complementary bearer-absent check: a mock gated on the Authorization header
/// must receive zero hits when `token = None`.
#[tokio::test]
async fn no_token_does_not_match_bearer_gated_mock() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex("^/v2/ontologies/metadata/"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(0) // Must NOT be called when token is None.
        .mount(&server)
        .await;

    // Fallback (no auth requirement).
    Mock::given(method("GET"))
        .and(path_regex("^/v2/ontologies/metadata/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_data_models(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "list_data_models(None) must succeed via fallback mock, got: {:?}",
        result
    );
    // The `expect(0)` on the bearer-gated mock is verified at drop.
}

// ---------------------------------------------------------------------------
// 500 → ServerError
// ---------------------------------------------------------------------------

#[tokio::test]
async fn server_error_500_returns_server_error_diagnostic() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex("^/v2/ontologies/metadata/"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_data_models(&uri, PROJECT_IRI, None)
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
// Malformed body → ServerError
// ---------------------------------------------------------------------------

/// A 200 response with an invalid JSON body must yield `ServerError` (not panic).
#[tokio::test]
async fn malformed_body_returns_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex("^/v2/ontologies/metadata/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("this is not json at all"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_data_models(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for malformed body");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "malformed body must map to Diagnostic::ServerError"
    );
}

// ---------------------------------------------------------------------------
// `data_model_name_from_iri` derivation
// ---------------------------------------------------------------------------

/// The `name` field is derived from the IRI, not a separate wire field.
/// This test guards the derivation: a standard `…/ontology/{code}/{name}/v2`
/// IRI must produce the last path segment before `/v2`.
#[tokio::test]
async fn name_derived_from_iri_correctly() {
    let server = MockServer::start().await;

    let body = json!({
        "@graph": [
            {
                "@id": "http://api.dasch.swiss/ontology/0801/beol/v2"
            }
        ]
    });

    mount_200(&server, body).await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_data_models(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    let models = result.expect("should succeed");
    assert_eq!(
        models[0].name, "beol",
        "name must be derived from the IRI (last path segment before /v2)"
    );
}

// ---------------------------------------------------------------------------
// URL construction — metadata_path() helper is coherent
// ---------------------------------------------------------------------------

/// Verify the URL path helper produces a non-empty path — this keeps the
/// helper honest without duplicating percent-encoding logic.
#[test]
fn metadata_path_helper_is_non_empty() {
    let p = metadata_path();
    assert!(
        p.starts_with("/v2/ontologies/metadata/"),
        "path must start with /v2/ontologies/metadata/"
    );
    assert!(
        p.len() > "/v2/ontologies/metadata/".len(),
        "path must include the encoded IRI"
    );
}
