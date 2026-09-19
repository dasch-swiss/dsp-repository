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
// Fixture IRIs use the `api.dasch.swiss/ontology/0801/beol` namespace to
// match real shapes verified against the live API. The endpoint is public:
// an unauthenticated call (token = None) omits the Authorization header
// entirely; an authenticated call sends `Authorization: Bearer <token>`.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use serde_json::json;
use wiremock::matchers::{header, header_exists, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

// A clearly synthetic token — never a real JWT.
const TOKEN: &str = "test-describe-token";

// The data-model IRI used across all tests (BEOL ontology on api.dasch.swiss).
const DATA_MODEL_IRI: &str = "http://api.dasch.swiss/ontology/0801/beol/v2";

// The BEOL namespace (what `beol:` expands to in the @context).
const BEOL_NS: &str = "http://api.dasch.swiss/ontology/0801/beol/v2#";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build the expected URL path for the allentities endpoint (percent-encoded IRI).
fn allentities_path() -> String {
    // The IRI is percent-encoded by `enc()` in the impl. We encode it the same
    // way: every non-alphanumeric character is encoded with `%XX`.
    let encoded: String = DATA_MODEL_IRI
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    format!("/v2/ontologies/allentities/{encoded}")
}

/// Mount a mock that returns `body` as a 200 JSON response on the exact
/// allentities path (so a regression dropping percent-encoding makes the mock
/// receive zero hits and wiremock returns 404 → test fails).
async fn mount_200(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("GET"))
        .and(path(allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

/// Build the minimal valid fixture body with a `@graph` and `@context`.
///
/// - 2 resource-types (one with label, one without)
/// - 1 standoff node (`isStandoffClass: true`, no `isResourceClass`) — must be filtered
/// - 1 property node (neither flag) — must be filtered
/// - Root `@id`, `rdfs:label`, `knora-api:lastModificationDate`
/// - `@context` mapping `beol` to the BEOL namespace
fn fixture_body() -> serde_json::Value {
    json!({
        "@id": DATA_MODEL_IRI,
        "rdfs:label": "The BEOL ontology",
        "knora-api:lastModificationDate": {
            "@value": "2024-05-27T13:43:26.233048Z",
            "@type": "xsd:dateTimeStamp"
        },
        "@graph": [
            {
                // resource-type WITH label — CURIE @id
                "@id": "beol:letter",
                "rdfs:label": "Letter",
                "knora-api:isResourceClass": true
            },
            {
                // resource-type WITHOUT label — CURIE @id (mixed-case name)
                "@id": "beol:Archive",
                "knora-api:isResourceClass": true
            },
            {
                // standoff node — must be filtered out
                "@id": "beol:StandoffBold",
                "rdfs:label": "Bold Standoff",
                "knora-api:isStandoffClass": true
            },
            {
                // property node (neither resource nor standoff) — must be filtered out
                "@id": "beol:hasText",
                "rdfs:label": "Has text"
            }
        ],
        "@context": {
            "beol": BEOL_NS,
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
            "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
            "xsd": "http://www.w3.org/2001/XMLSchema#"
        }
    })
}

// ---------------------------------------------------------------------------
// Happy path — 2 resource-types, filter, CURIE expansion, sort
// ---------------------------------------------------------------------------

/// Full happy-path test:
/// - Exactly 2 resource-types returned (standoff + property filtered out).
/// - IRIs expanded via `@context` (`beol:letter` → `…/beol/v2#letter`).
/// - `label` Option honoured (one Some, one None).
/// - Sorted by name (`Archive` < `letter` alphabetically).
/// - Root `name`/`label`/`last_modified` parsed correctly.
#[tokio::test]
async fn happy_path_returns_two_resource_types_filtered_sorted_expanded() {
    let server = MockServer::start().await;

    mount_200(&server, fixture_body()).await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_data_model(&uri, DATA_MODEL_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let detail = result.unwrap();

    // Root fields
    assert_eq!(detail.name, "beol", "name must be derived from IRI");
    assert_eq!(detail.iri, DATA_MODEL_IRI, "iri must match root @id");
    assert_eq!(
        detail.label.as_deref(),
        Some("The BEOL ontology"),
        "label must be parsed from rdfs:label"
    );
    // Assert the FULL RFC3339 value is preserved verbatim (lossless passthrough),
    // not merely the date prefix — guards against a truncation bug dropping the time.
    assert_eq!(
        detail.last_modified.as_deref(),
        Some("2024-05-27T13:43:26.233048Z"),
        "last_modified must be the raw RFC3339 string from lastModificationDate.@value"
    );

    // Resource-types: exactly 2 (standoff + property filtered out)
    assert_eq!(
        detail.resource_types.len(),
        2,
        "exactly 2 resource-types (standoff and property filtered); got {:?}",
        detail.resource_types
    );

    // Sorted by name: `Archive` < `letter`
    assert_eq!(
        detail.resource_types[0].name, "Archive",
        "first resource-type (sorted) must be Archive"
    );
    assert_eq!(
        detail.resource_types[1].name, "letter",
        "second resource-type (sorted) must be letter"
    );

    // CURIE expansion via @context
    assert_eq!(
        detail.resource_types[0].iri,
        format!("{BEOL_NS}Archive"),
        "Archive IRI must be expanded from CURIE beol:Archive"
    );
    assert_eq!(
        detail.resource_types[1].iri,
        format!("{BEOL_NS}letter"),
        "letter IRI must be expanded from CURIE beol:letter"
    );

    // label Option: Archive has None (absent in fixture), letter has Some
    assert_eq!(
        detail.resource_types[0].label, None,
        "Archive must have label None (absent in fixture)"
    );
    assert_eq!(
        detail.resource_types[1].label.as_deref(),
        Some("Letter"),
        "letter must have label Some(\"Letter\")"
    );
}

// ---------------------------------------------------------------------------
// Empty @graph
// ---------------------------------------------------------------------------

/// When `@graph` is empty (or absent), `resource_types` must be empty.
/// Root fields are still parsed from the top-level document.
#[tokio::test]
async fn empty_graph_returns_zero_resource_types() {
    let server = MockServer::start().await;

    let body = json!({
        "@id": DATA_MODEL_IRI,
        "rdfs:label": "Empty ontology",
        "@graph": [],
        "@context": {
            "beol": BEOL_NS
        }
    });
    mount_200(&server, body).await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_data_model(&uri, DATA_MODEL_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let detail = result.unwrap();
    assert!(detail.resource_types.is_empty(), "empty @graph must yield zero resource_types");
    assert_eq!(detail.name, "beol");
    assert_eq!(detail.label.as_deref(), Some("Empty ontology"));
}

// ---------------------------------------------------------------------------
// Bearer present
// ---------------------------------------------------------------------------

/// When `token = Some(TOKEN)` is passed, the request must carry an
/// `Authorization: Bearer <TOKEN>` header.
#[tokio::test]
async fn bearer_present_when_token_is_some() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(allentities_path()))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "@id": DATA_MODEL_IRI,
            "@graph": [],
            "@context": {}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_data_model(&uri, DATA_MODEL_IRI, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok when token is Some, got: {:?}", result);

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    let auth = received[0]
        .headers
        .get("authorization")
        .expect("Authorization header must be present when token is Some");
    assert_eq!(
        auth.to_str().expect("header should be valid UTF-8"),
        format!("Bearer {TOKEN}"),
        "bearer token must match the value passed to describe_data_model"
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
        .and(path_regex("^/v2/ontologies/allentities/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "@id": DATA_MODEL_IRI,
            "@graph": [],
            "@context": {}
        })))
        .mount(&server)
        .await;

    let uri = server.uri();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_data_model(&uri, DATA_MODEL_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    let received = server.received_requests().await.expect("request recording should be enabled");
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
        .and(path_regex("^/v2/ontologies/allentities/"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "@id": DATA_MODEL_IRI,
            "@graph": [],
            "@context": {}
        })))
        .expect(0) // Must NOT be called when token is None.
        .mount(&server)
        .await;

    // Fallback (no auth requirement).
    Mock::given(method("GET"))
        .and(path_regex("^/v2/ontologies/allentities/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "@id": DATA_MODEL_IRI,
            "@graph": [],
            "@context": {}
        })))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_data_model(&uri, DATA_MODEL_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "describe_data_model(None) must succeed via fallback mock, got: {:?}",
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
        .and(path_regex("^/v2/ontologies/allentities/"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_data_model(&uri, DATA_MODEL_IRI, None)
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
// 404 → ServerError (via map_unexpected_status — no special handling)
// ---------------------------------------------------------------------------

/// A 404 from allentities maps to `ServerError` (not `NotFound`).
/// The action already matched the data-model IRI via `list_data_models`; a 404
/// here is an unexpected server state (TOCTOU), not a user-facing "not found".
/// This mirrors `list_data_models`'s uniform `map_unexpected_status` behaviour.
#[tokio::test]
async fn not_found_404_returns_server_error_diagnostic() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex("^/v2/ontologies/allentities/"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_data_model(&uri, DATA_MODEL_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 404 response");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "404 must map to Diagnostic::ServerError (not NotFound) for allentities"
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
        .and(path_regex("^/v2/ontologies/allentities/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("this is not json at all"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_data_model(&uri, DATA_MODEL_IRI, None)
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
// URL construction helper is coherent
// ---------------------------------------------------------------------------

/// Verify the URL path helper produces a non-empty path — this keeps the
/// helper honest without duplicating percent-encoding logic.
#[test]
fn allentities_path_helper_is_non_empty() {
    let p = allentities_path();
    assert!(
        p.starts_with("/v2/ontologies/allentities/"),
        "path must start with /v2/ontologies/allentities/"
    );
    assert!(
        p.len() > "/v2/ontologies/allentities/".len(),
        "path must include the encoded IRI"
    );
}
