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
// Fixture IRIs use the `api.dasch.swiss/ontology/0001/test/v2` namespace with
// a sibling `sib` ontology, to test cross-DM field resolution (Decision 9).
// The bearer-present/absent assertions mirror `data_model_describe_http.rs`.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::{Cardinality, Representation, ValueType};
use serde_json::json;
use wiremock::matchers::{header, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

// A clearly synthetic token — never a real JWT.
const TOKEN: &str = "test-resource-type-token";

// The data-model IRI for all primary tests.
const DATA_MODEL_IRI: &str = "http://0.0.0.0:3333/ontology/0001/test/v2";

// The test namespace (what `test:` expands to in the @context).
const TEST_NS: &str = "http://0.0.0.0:3333/ontology/0001/test/v2#";

// The sibling (sib) ontology IRI and namespace.
const SIB_IRI: &str = "http://0.0.0.0:3333/ontology/0001/sib/v2";
const SIB_NS: &str = "http://0.0.0.0:3333/ontology/0001/sib/v2#";

// ---------------------------------------------------------------------------
// Helpers — path construction
// ---------------------------------------------------------------------------

/// Build the expected URL path for the allentities endpoint (percent-encoded IRI).
fn allentities_path(iri: &str) -> String {
    let encoded: String = iri
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

fn test_allentities_path() -> String {
    allentities_path(DATA_MODEL_IRI)
}

fn sib_allentities_path() -> String {
    allentities_path(SIB_IRI)
}

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

/// The context for the primary `test` ontology fixture (always the same).
fn test_context() -> serde_json::Value {
    json!({
        "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
        "rdfs":      "http://www.w3.org/2000/01/rdf-schema#",
        "owl":       "http://www.w3.org/2002/07/owl#",
        "salsah-gui": "http://api.knora.org/ontology/salsah-gui/v2#",
        "test":      TEST_NS,
        "sib":       SIB_NS
    })
}

/// `@graph` entry for the `test:Doc` resource class. This is the main happy-path
/// class exercised in most tests.
///
/// `rdfs:subClassOf` contains:
/// 1. A project superclass ref `test:Base`.
/// 2. A `knora-api` system superclass ref (dropped from super_types).
/// 3. A file-value restriction (still-image; sets representation; is_builtin).
/// 4. `test:hasTitle` — project text field, guiOrder=1.
/// 5. `test:linksTo` — project link field, guiOrder=2.
/// 6. `test:linksToValue` — link-value twin (dropped).
/// 7. `knora-api:arkUrl` — system field (is_builtin=true).
fn doc_class() -> serde_json::Value {
    json!({
        "@id": "test:Doc",
        "@type": "owl:Class",
        "knora-api:isResourceClass": true,
        "rdfs:label": "Document",
        "rdfs:subClassOf": [
            // project superclass ref → appears in super_types as "Base"
            { "@id": "test:Base" },
            // system superclass ref → dropped from super_types
            { "@id": "knora-api:StillImageRepresentation" },
            // file-value restriction → representation = StillImage (is_builtin; no guiOrder)
            {
                "@type": "owl:Restriction",
                "owl:onProperty": { "@id": "knora-api:hasStillImageFileValue" },
                "owl:cardinality": 1,
                "knora-api:isInherited": true
            },
            // project text field
            {
                "@type": "owl:Restriction",
                "owl:onProperty": { "@id": "test:hasTitle" },
                "owl:cardinality": 1,
                "salsah-gui:guiOrder": 1
            },
            // project link field
            {
                "@type": "owl:Restriction",
                "owl:onProperty": { "@id": "test:linksTo" },
                "owl:minCardinality": 0,
                "salsah-gui:guiOrder": 2
            },
            // link-value twin — must be DROPPED from the field list
            {
                "@type": "owl:Restriction",
                "owl:onProperty": { "@id": "test:linksToValue" },
                "owl:minCardinality": 0
            },
            // system field (knora-api) — is_builtin=true; no guiOrder → u32::MAX
            {
                "@type": "owl:Restriction",
                "owl:onProperty": { "@id": "knora-api:arkUrl" },
                "owl:cardinality": 1,
                "knora-api:isInherited": true
            }
        ]
    })
}

/// Property node for `test:hasTitle` (text field, guiOrder on restriction).
fn prop_has_title() -> serde_json::Value {
    json!({
        "@id": "test:hasTitle",
        "knora-api:isResourceProperty": true,
        "knora-api:objectType": { "@id": "knora-api:TextValue" },
        "rdfs:label": "Title"
    })
}

/// Property node for `test:linksTo` (link field; objectType = target resource class).
fn prop_links_to() -> serde_json::Value {
    json!({
        "@id": "test:linksTo",
        "knora-api:isResourceProperty": true,
        "knora-api:isLinkProperty": true,
        "knora-api:objectType": { "@id": "test:Target" },
        "rdfs:label": "Links to"
    })
}

/// Property node for `test:linksToValue` (reification twin — isLinkValueProperty=true).
fn prop_links_to_value() -> serde_json::Value {
    json!({
        "@id": "test:linksToValue",
        "knora-api:isLinkValueProperty": true,
        "knora-api:objectType": { "@id": "knora-api:LinkValue" },
        "rdfs:label": "Links to"
    })
}

/// Full happy-path fixture for the `test` ontology.
///
/// Includes: `test:Doc` class, `test:Base` class (superclass), `test:hasTitle`,
/// `test:linksTo`, `test:linksToValue` property nodes. Note: `knora-api:arkUrl`
/// property node is intentionally ABSENT (foreign system prop, as in real responses).
fn happy_fixture() -> serde_json::Value {
    json!({
        "@id": DATA_MODEL_IRI,
        "rdfs:label": "Test ontology",
        "@graph": [
            doc_class(),
            // Base class (superclass ref target — present as a resource class)
            {
                "@id": "test:Base",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Base type"
            },
            prop_has_title(),
            prop_links_to(),
            prop_links_to_value()
        ],
        "@context": test_context()
    })
}

// ---------------------------------------------------------------------------
// Test 1: Happy path — query `test:Doc`
// ---------------------------------------------------------------------------

/// Happy path: querying `test:Doc` returns a well-formed `ResourceTypeDetail`.
///
/// Assertions:
/// - name = "Doc", label = Some("Document"), representation = Some(StillImage)
/// - super_types = ["Base"] (system super `knora-api:StillImageRepresentation` dropped)
/// - project fields: hasTitle (text, One) then linksTo (link → Target, ZeroOrMore) (ordered by
///   guiOrder 1, 2)
/// - link-value twin `linksToValue` is DROPPED
/// - built-in `arkUrl` IS present with is_builtin=true (client returns full set)
/// - `linksTo.link_target == Some("Target")`
#[tokio::test]
async fn happy_path_describe_doc_resource_type() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(test_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(happy_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource_type(&uri, DATA_MODEL_IRI, "Doc", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let detail = result.unwrap();

    // Identity
    assert_eq!(detail.name, "Doc", "name must be the class local name");
    assert_eq!(detail.label.as_deref(), Some("Document"), "label must match rdfs:label");
    assert_eq!(detail.data_model, "test", "data_model must be derived from IRI");
    assert_eq!(detail.iri, format!("{TEST_NS}Doc"), "IRI must be expanded");

    // Representation
    assert_eq!(
        detail.representation,
        Some(Representation::StillImage),
        "representation must be StillImage (detected from hasStillImageFileValue restriction)"
    );

    // super_types: only project non-system supers
    assert_eq!(
        detail.super_types,
        vec!["Base"],
        "super_types must contain only the project superclass; knora-api super dropped"
    );

    // Field ordering: guiOrder=1 → hasTitle, guiOrder=2 → linksTo;
    // system arkUrl has no guiOrder → sorts last (u32::MAX sentinel).
    // twin linksToValue must be dropped entirely.
    let project_fields: Vec<_> = detail.fields.iter().filter(|f| !f.is_builtin).collect();

    assert_eq!(
        project_fields.len(),
        2,
        "exactly 2 project fields expected (hasTitle + linksTo); twin linksToValue dropped"
    );

    let has_title = &project_fields[0];
    assert_eq!(has_title.name, "hasTitle", "first project field must be hasTitle (guiOrder=1)");
    assert_eq!(has_title.value_type, ValueType::Text, "hasTitle must have value_type Text");
    assert_eq!(has_title.cardinality, Cardinality::One, "hasTitle must have cardinality One");
    assert_eq!(has_title.label.as_deref(), Some("Title"), "hasTitle label");
    assert!(!has_title.is_builtin, "hasTitle must not be builtin");
    assert_eq!(has_title.link_target, None, "text field must have link_target None");

    let links_to = &project_fields[1];
    assert_eq!(links_to.name, "linksTo", "second project field must be linksTo (guiOrder=2)");
    assert_eq!(links_to.value_type, ValueType::Link, "linksTo must have value_type Link");
    assert_eq!(
        links_to.link_target.as_deref(),
        Some("Target"),
        "linksTo.link_target must be Some(\"Target\")"
    );
    assert_eq!(
        links_to.cardinality,
        Cardinality::ZeroOrMore,
        "linksTo must have cardinality ZeroOrMore"
    );
    assert!(!links_to.is_builtin, "linksTo must not be builtin");

    // Twin linksToValue must be dropped (not in the field list at all).
    let twin_present = detail.fields.iter().any(|f| f.name == "linksToValue");
    assert!(
        !twin_present,
        "linksToValue (link-value reification twin) must be dropped from the field list"
    );

    // Built-in arkUrl must be present with is_builtin=true.
    let ark = detail.fields.iter().find(|f| f.name == "arkUrl");
    assert!(
        ark.is_some(),
        "arkUrl system field must be present in the client result (action filters later)"
    );
    let ark = ark.unwrap();
    assert!(ark.is_builtin, "arkUrl must be marked is_builtin=true");
    assert_eq!(ark.data_model, None, "system field data_model must be None");
}

// ---------------------------------------------------------------------------
// Test 2: Cardinality + value-type mapping
// ---------------------------------------------------------------------------

/// Tests all four cardinality variants and several value-type mappings.
///
/// Field set:
/// - `test:intField`    objectType IntValue,   owl:cardinality=1  → One, integer
/// - `test:dateField`   objectType DateValue,  owl:maxCardinality=1 → ZeroOrOne, date
/// - `test:uriField`    objectType UriValue,   owl:minCardinality=1 → OneOrMore, uri
/// - (hasTitle from happy fixture provides ZeroOrMore / Text via its own test)
#[tokio::test]
async fn cardinality_and_value_type_mapping() {
    let server = MockServer::start().await;

    let fixture = json!({
        "@id": DATA_MODEL_IRI,
        "@graph": [
            // Target class with restrictions for each cardinality + value-type variant
            {
                "@id": "test:TypeB",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Type B",
                "rdfs:subClassOf": [
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "test:intField" },
                        "owl:cardinality": 1,
                        "salsah-gui:guiOrder": 1
                    },
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "test:dateField" },
                        "owl:maxCardinality": 1,
                        "salsah-gui:guiOrder": 2
                    },
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "test:uriField" },
                        "owl:minCardinality": 1,
                        "salsah-gui:guiOrder": 3
                    }
                ]
            },
            // Property nodes
            {
                "@id": "test:intField",
                "knora-api:isResourceProperty": true,
                "knora-api:objectType": { "@id": "knora-api:IntValue" },
                "rdfs:label": "Integer field"
            },
            {
                "@id": "test:dateField",
                "knora-api:isResourceProperty": true,
                "knora-api:objectType": { "@id": "knora-api:DateValue" },
                "rdfs:label": "Date field"
            },
            {
                "@id": "test:uriField",
                "knora-api:isResourceProperty": true,
                "knora-api:objectType": { "@id": "knora-api:UriValue" },
                "rdfs:label": "URI field"
            }
        ],
        "@context": test_context()
    });

    Mock::given(method("GET"))
        .and(path(test_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource_type(&uri, DATA_MODEL_IRI, "TypeB", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let detail = result.unwrap();

    assert_eq!(detail.name, "TypeB");

    let fields: Vec<_> = detail.fields.iter().filter(|f| !f.is_builtin).collect();
    assert_eq!(fields.len(), 3, "expected 3 project fields");

    // guiOrder=1: intField — One, integer
    assert_eq!(fields[0].name, "intField");
    assert_eq!(fields[0].value_type, ValueType::Integer, "intField must map to Integer");
    assert_eq!(fields[0].cardinality, Cardinality::One, "owl:cardinality=1 → One");
    assert_eq!(fields[0].link_target, None, "non-link field must have None link_target");

    // guiOrder=2: dateField — ZeroOrOne, date
    assert_eq!(fields[1].name, "dateField");
    assert_eq!(fields[1].value_type, ValueType::Date, "dateField must map to Date");
    assert_eq!(
        fields[1].cardinality,
        Cardinality::ZeroOrOne,
        "owl:maxCardinality=1 → ZeroOrOne"
    );

    // guiOrder=3: uriField — OneOrMore, uri
    assert_eq!(fields[2].name, "uriField");
    assert_eq!(fields[2].value_type, ValueType::Uri, "uriField must map to Uri");
    assert_eq!(
        fields[2].cardinality,
        Cardinality::OneOrMore,
        "owl:minCardinality=1 → OneOrMore"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Bearer present / absent
// ---------------------------------------------------------------------------

/// When `token = Some(TOKEN)` is passed, the request must carry
/// `Authorization: Bearer <TOKEN>`.
#[tokio::test]
async fn bearer_present_when_token_is_some() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(test_allentities_path()))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(happy_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource_type(&uri, DATA_MODEL_IRI, "Doc", Some(TOKEN))
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
        "bearer token must match the value passed to describe_resource_type"
    );
}

/// When `token = None` is passed, NO `Authorization` header must be sent.
/// Guards against a future refactor accidentally sending an unconditional bearer.
#[tokio::test]
async fn bearer_absent_when_token_is_none() {
    let server = MockServer::start().await;

    // Use a fallback mock (no auth requirement) so the call succeeds.
    Mock::given(method("GET"))
        .and(path_regex("^/v2/ontologies/allentities/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "@id": DATA_MODEL_IRI,
            "@graph": [
                {
                    "@id": "test:Doc",
                    "knora-api:isResourceClass": true,
                    "rdfs:label": "Document",
                    "rdfs:subClassOf": []
                }
            ],
            "@context": test_context()
        })))
        .mount(&server)
        .await;

    let uri = server.uri();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource_type(&uri, DATA_MODEL_IRI, "Doc", None)
    })
    .join()
    .expect("blocking thread should not panic");

    let received = server.received_requests().await.expect("request recording should be enabled");
    // At least one request (the primary allentities call).
    assert!(!received.is_empty(), "at least one request must have been made");
    // The first request (primary allentities) must not have an Authorization header.
    assert!(
        received[0].headers.get("authorization").is_none(),
        "Authorization header must be ABSENT when token is None"
    );
}

// ---------------------------------------------------------------------------
// Test 4: Resource-type not found
// ---------------------------------------------------------------------------

/// Querying a name absent from the graph returns `Err(Diagnostic::NotFound(_))`.
/// The error message must NOT contain "resource-type list" — that hint is the
/// action's responsibility (ADR-0001 boundary).
#[tokio::test]
async fn resource_type_not_found_returns_not_found_diagnostic() {
    let server = MockServer::start().await;

    // Fixture has only `test:Doc`; we query `test:Unknown`.
    Mock::given(method("GET"))
        .and(path(test_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(happy_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource_type(&uri, DATA_MODEL_IRI, "Unknown", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for absent resource-type");
    match result.unwrap_err() {
        Diagnostic::NotFound(msg) => {
            // The message must NOT mention "resource-type list" — that hint belongs
            // to the action layer, not the client (ADR-0001).
            assert!(
                !msg.contains("resource-type list"),
                "client NotFound message must not contain 'resource-type list'; got: {msg}"
            );
        }
        other => panic!("expected Diagnostic::NotFound, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 5: Server error (500)
// ---------------------------------------------------------------------------

/// A 500 response from allentities returns `Err(Diagnostic::ServerError(_))`.
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
        client.describe_resource_type(&uri, DATA_MODEL_IRI, "Doc", None)
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
// Test 6: Two-ontology sibling fetch (Decision 9)
// ---------------------------------------------------------------------------

/// When a class restriction references a property from a sibling ontology (`sib:`),
/// the client fetches the sibling's allentities and resolves the property node.
///
/// Scenario:
/// - `test:Doc` class has a restriction on `sib:reusedField` (no node in test graph).
/// - A second mock endpoint for `sib` returns a graph with `sib:reusedField`.
/// - Assert: the returned field has value_type uri, label Some("Reused"), data_model Some("sib"),
///   and is NOT built-in.
#[tokio::test]
async fn two_ontology_sibling_fetch_resolves_field() {
    let server = MockServer::start().await;

    // Primary ontology fixture: Doc class with sib:reusedField restriction.
    let primary = json!({
        "@id": DATA_MODEL_IRI,
        "@graph": [
            {
                "@id": "test:Doc",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Document",
                "rdfs:subClassOf": [
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "test:hasTitle" },
                        "owl:cardinality": 1,
                        "salsah-gui:guiOrder": 1
                    },
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "sib:reusedField" },
                        "owl:minCardinality": 0,
                        "salsah-gui:guiOrder": 2
                    }
                ]
            },
            prop_has_title()
            // Note: sib:reusedField property node is absent here — must be fetched from sib
        ],
        "@context": test_context()
    });

    // Sibling ontology fixture: contains the sib:reusedField property node.
    let sibling = json!({
        "@id": SIB_IRI,
        "@graph": [
            {
                "@id": "sib:reusedField",
                "knora-api:isResourceProperty": true,
                "knora-api:objectType": { "@id": "knora-api:UriValue" },
                "rdfs:label": "Reused"
            }
        ],
        "@context": {
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
            "sib": SIB_NS
        }
    });

    // Mount both mocks — each expects exactly 1 hit.
    Mock::given(method("GET"))
        .and(path(test_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(primary))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(sib_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(sibling))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource_type(&uri, DATA_MODEL_IRI, "Doc", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok with sibling fetch, got: {:?}", result);
    let detail = result.unwrap();

    // Find `reusedField` in the returned fields.
    let reused = detail
        .fields
        .iter()
        .find(|f| f.name == "reusedField")
        .expect("reusedField from sibling ontology must be present in the field list");

    assert_eq!(
        reused.value_type,
        ValueType::Uri,
        "reusedField must have value_type Uri (resolved from sib ontology)"
    );
    assert_eq!(
        reused.label.as_deref(),
        Some("Reused"),
        "reusedField must have label Some(\"Reused\") from sib property node"
    );
    assert_eq!(
        reused.data_model.as_deref(),
        Some("sib"),
        "reusedField.data_model must be Some(\"sib\") — the CURIE prefix"
    );
    assert!(
        !reused.is_builtin,
        "reusedField from a project sibling ontology must not be marked builtin"
    );
    assert_eq!(
        reused.cardinality,
        Cardinality::ZeroOrMore,
        "reusedField must have cardinality ZeroOrMore (owl:minCardinality=0)"
    );
}

// ---------------------------------------------------------------------------
// Test 6b: Representation detected from restriction, NOT from superclass ref
// ---------------------------------------------------------------------------

/// R8 isolation: representation is detected from the file-value RESTRICTION
/// (`owl:onProperty knora-api:hasMovingImageFileValue`), NOT from a
/// `knora-api:*Representation` superclass ref.
///
/// Fixture: `test:Video` class has ONLY a project superclass ref (`test:Base`)
/// and NO `knora-api:MovingImageRepresentation` superclass ref, but it DOES have
/// an `owl:Restriction` on `knora-api:hasMovingImageFileValue` with cardinality=1
/// and `knora-api:isInherited=true`. This mirrors the real transitive case where
/// the representation super is NOT in the class's own `subClassOf`.
///
/// Assert: `representation == Some(Representation::MovingImage)`.
#[tokio::test]
async fn representation_detected_from_restriction_not_superclass_ref() {
    let server = MockServer::start().await;

    let fixture = json!({
        "@id": DATA_MODEL_IRI,
        "@graph": [
            {
                "@id": "test:Video",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Video",
                "rdfs:subClassOf": [
                    // Project superclass ref ONLY — no knora-api:*Representation here
                    { "@id": "test:Base" },
                    // MovingImage file-value restriction — this is how representation is detected
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "knora-api:hasMovingImageFileValue" },
                        "owl:cardinality": 1,
                        "knora-api:isInherited": true
                    }
                ]
            },
            {
                "@id": "test:Base",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Base type"
            }
        ],
        "@context": test_context()
    });

    Mock::given(method("GET"))
        .and(path(test_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource_type(&uri, DATA_MODEL_IRI, "Video", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let detail = result.unwrap();

    assert_eq!(detail.name, "Video");

    // CRITICAL: representation must be MovingImage, detected from the restriction
    // on `knora-api:hasMovingImageFileValue`, NOT from a representation superclass ref
    // (which is absent in this fixture).
    assert_eq!(
        detail.representation,
        Some(Representation::MovingImage),
        "representation must be MovingImage — detected from hasMovingImageFileValue restriction, \
         not from a knora-api:*Representation superclass ref (which is absent in this fixture)"
    );

    // super_types: only the project super (test:Base), knora-api system supers absent
    assert_eq!(
        detail.super_types,
        vec!["Base"],
        "super_types must contain only the project superclass"
    );
}

// ---------------------------------------------------------------------------
// Test 7: Sibling fetch fails (non-fatal)
// ---------------------------------------------------------------------------

/// When the sibling ontology fetch fails (server error), the field is degraded
/// but the command still returns Ok. The degraded field:
/// - is still present in the field list
/// - carries cardinality + name + data_model Some("sib")
/// - has value_type = ValueType::Other("—") (sentinel for unresolved node)
/// - has label = None
/// - is NOT marked as an error at the command level (command still succeeds)
#[tokio::test]
async fn sibling_fetch_fails_field_degrades_but_command_succeeds() {
    let server = MockServer::start().await;

    // Same primary fixture as test 6 — Doc with sib:reusedField restriction.
    let primary = json!({
        "@id": DATA_MODEL_IRI,
        "@graph": [
            {
                "@id": "test:Doc",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Document",
                "rdfs:subClassOf": [
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "test:hasTitle" },
                        "owl:cardinality": 1,
                        "salsah-gui:guiOrder": 1
                    },
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "sib:reusedField" },
                        "owl:minCardinality": 0,
                        "salsah-gui:guiOrder": 2
                    }
                ]
            },
            prop_has_title()
        ],
        "@context": test_context()
    });

    // Primary ontology mock — succeeds.
    Mock::given(method("GET"))
        .and(path(test_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(primary))
        .expect(1)
        .mount(&server)
        .await;

    // Sibling ontology mock — returns 500 (fetch fails).
    Mock::given(method("GET"))
        .and(path(sib_allentities_path()))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource_type(&uri, DATA_MODEL_IRI, "Doc", None)
    })
    .join()
    .expect("blocking thread should not panic");

    // Command must still succeed — sibling fetch failure is non-fatal (R5).
    assert!(
        result.is_ok(),
        "sibling fetch failure must be non-fatal; command must return Ok, got: {:?}",
        result
    );

    let detail = result.unwrap();

    // The degraded field must still be present.
    let reused = detail
        .fields
        .iter()
        .find(|f| f.name == "reusedField")
        .expect("reusedField must still appear even when sibling fetch fails (best-effort)");

    // Degraded field semantics:
    // - value_type = Other("—") — sentinel for an unresolved node
    // - label = None — property node unavailable
    // - data_model = Some("sib") — source DM known from CURIE prefix
    // - cardinality preserved from the restriction
    assert_eq!(
        reused.value_type,
        ValueType::Other("—".to_string()),
        "degraded field must have value_type Other(\"—\") (unresolved node sentinel)"
    );
    assert_eq!(
        reused.label, None,
        "degraded field label must be None (property node unavailable)"
    );
    assert_eq!(
        reused.data_model.as_deref(),
        Some("sib"),
        "degraded field must still carry data_model Some(\"sib\") — source DM from CURIE prefix"
    );
    assert_eq!(
        reused.cardinality,
        Cardinality::ZeroOrMore,
        "cardinality must be preserved even for a degraded field"
    );
    assert!(
        !reused.is_builtin,
        "sib field must not be marked builtin even in degraded state"
    );
}

// ---------------------------------------------------------------------------
// Test 8: Missing prefix in @context degrades gracefully (non-fatal, no panic)
// ---------------------------------------------------------------------------

/// When a restriction's onProperty CURIE prefix (`weird:`) is NOT in `@context`,
/// the sibling-fetch loop skips that prefix with a warning (no panic, no Err).
/// The field is still present in the result — degraded to `value_type Other("—")`
/// — and `data_model == Some("weird")` is derived from the CURIE prefix alone.
///
/// This confirms the production code's skip logic (lines where the prefix lookup
/// returns `None` → `continue`) without panicking or hard-erroring.
#[tokio::test]
async fn field_with_prefix_absent_from_context_degrades_gracefully() {
    let server = MockServer::start().await;

    // `weird` is NOT in @context — no resolution possible.
    // Note: we deliberately do NOT add "weird" to the context; that's the point.
    let fixture = json!({
        "@id": DATA_MODEL_IRI,
        "@graph": [
            {
                "@id": "test:Doc",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Document",
                "rdfs:subClassOf": [
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "weird:someField" },
                        "owl:minCardinality": 0,
                        "salsah-gui:guiOrder": 1
                    }
                    // No property node for weird:someField — prefix absent from @context
                ]
            }
        ],
        // @context does NOT contain "weird" — that's the scenario being tested
        "@context": {
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
            "rdfs":      "http://www.w3.org/2000/01/rdf-schema#",
            "owl":       "http://www.w3.org/2002/07/owl#",
            "salsah-gui": "http://api.knora.org/ontology/salsah-gui/v2#",
            "test":      TEST_NS
            // "weird" intentionally absent
        }
    });

    Mock::given(method("GET"))
        .and(path(test_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource_type(&uri, DATA_MODEL_IRI, "Doc", None)
    })
    .join()
    .expect("blocking thread should not panic");

    // Must return Ok — missing prefix is non-fatal (no panic, no hard error).
    assert!(result.is_ok(), "missing @context prefix must not cause Err; got: {:?}", result);
    let detail = result.unwrap();

    // The degraded field must still be present.
    let some_field = detail
        .fields
        .iter()
        .find(|f| f.name == "someField")
        .expect("someField must be present even when its prefix is absent from @context");

    // Degraded semantics: unknown prefix → can't resolve value type
    assert_eq!(
        some_field.value_type,
        ValueType::Other("—".to_string()),
        "field with unresolvable prefix must degrade to ValueType::Other(\"—\")"
    );

    // data_model is derived from the CURIE prefix alone — even without a context entry
    assert_eq!(
        some_field.data_model.as_deref(),
        Some("weird"),
        "data_model must be Some(\"weird\") derived from the CURIE prefix"
    );

    // label is None — no property node was found
    assert_eq!(
        some_field.label, None,
        "degraded field label must be None (property node not found)"
    );

    // cardinality is preserved from the restriction
    assert_eq!(
        some_field.cardinality,
        Cardinality::ZeroOrMore,
        "cardinality must be preserved from the restriction"
    );

    // is_builtin must be false — "weird" is not a system prefix
    assert!(
        !some_field.is_builtin,
        "field with unknown project prefix must not be marked builtin"
    );
}

// ---------------------------------------------------------------------------
// Test 9: Bearer token forwarded to the sibling ontology fetch (R10)
// ---------------------------------------------------------------------------

/// When `token = Some(TOKEN)` is passed, the sibling ontology fetch must also
/// carry `Authorization: Bearer <TOKEN>`. This verifies R10 (bearer token
/// forwarded to sibling fetches, not just the primary allentities call).
///
/// The test mirrors the bearer-present assertion from Test 3, but applies it
/// to the SIBLING mock endpoint.
#[tokio::test]
async fn bearer_token_forwarded_to_sibling_fetch() {
    let server = MockServer::start().await;

    // Primary ontology: Doc with sib:reusedField restriction (same as Test 6).
    let primary = json!({
        "@id": DATA_MODEL_IRI,
        "@graph": [
            {
                "@id": "test:Doc",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Document",
                "rdfs:subClassOf": [
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "sib:reusedField" },
                        "owl:minCardinality": 0,
                        "salsah-gui:guiOrder": 1
                    }
                ]
            }
        ],
        "@context": test_context()
    });

    // Sibling ontology fixture.
    let sibling = json!({
        "@id": SIB_IRI,
        "@graph": [
            {
                "@id": "sib:reusedField",
                "knora-api:isResourceProperty": true,
                "knora-api:objectType": { "@id": "knora-api:UriValue" },
                "rdfs:label": "Reused"
            }
        ],
        "@context": {
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
            "sib": SIB_NS
        }
    });

    // Primary mock: requires the bearer token.
    Mock::given(method("GET"))
        .and(path(test_allentities_path()))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(primary))
        .expect(1)
        .mount(&server)
        .await;

    // Sibling mock: also requires the bearer token — this is what we're testing.
    Mock::given(method("GET"))
        .and(path(sib_allentities_path()))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(sibling))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource_type(&uri, DATA_MODEL_IRI, "Doc", Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "expected Ok when token is Some and sibling fetch is required, got: {:?}",
        result
    );

    // Verify both requests received the bearer token.
    let received = server.received_requests().await.expect("request recording should be enabled");

    // Both mocks expect exactly 1 call each; wiremock will fail if either misses.
    // Additionally verify each received request carries the Authorization header.
    assert_eq!(
        received.len(),
        2,
        "exactly two requests must have been made (primary + sibling)"
    );
    for req in &received {
        let auth = req
            .headers
            .get("authorization")
            .expect("Authorization header must be present on all requests when token is Some");
        assert_eq!(
            auth.to_str().expect("header should be valid UTF-8"),
            format!("Bearer {TOKEN}"),
            "bearer token must match on every request (primary and sibling)"
        );
    }

    // The field must be resolved from the sibling (not degraded).
    let detail = result.unwrap();
    let reused = detail
        .fields
        .iter()
        .find(|f| f.name == "reusedField")
        .expect("reusedField must be present when sibling fetch succeeds");
    assert_eq!(
        reused.value_type,
        ValueType::Uri,
        "reusedField must resolve to Uri (sibling fetch succeeded with token)"
    );
}
