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
// This test exercises `data_model_structure` (Step 3). One `allentities`
// fixture is used — no sibling fetch. Assertions cover:
//   - project link field → same-DM target (target_data_model = Some(this_dm))
//   - project link field → sibling-DM target (target_data_model = Some(sibling))
//   - project link field → built-in target (is_builtin=false, target_data_model=None)
//   - …Value link-value twin DROPPED
//   - project superclass (is_builtin=false, target_data_model=Some(dm))
//   - system superclass knora-base:Resource (is_builtin=true, target_data_model=None)
//   - D6 sort order (source, kind, field, target)
//   - bearer token forwarded (Authorization header)
//   - exactly ONE request hits the server (no sibling fetch)
//   - absent cross-DM link property node → SKIP (not an error)

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::{DataModelStructure, Relation, RelationKind};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// A clearly synthetic token — never a real JWT.
const TOKEN: &str = "test-structure-token";

// The data-model IRI for the primary tests.
const DATA_MODEL_IRI: &str = "http://0.0.0.0:3333/ontology/0001/myonto/v2";

// The `myonto` namespace.
const MY_NS: &str = "http://0.0.0.0:3333/ontology/0001/myonto/v2#";

// A sibling DM prefix present in the @context but whose property nodes are
// absent from this ontology's graph (v1 skip).
const SIB_NS: &str = "http://0.0.0.0:3333/ontology/0001/sibling/v2#";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Percent-encode the data-model IRI the same way `enc()` does in http.rs.
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

fn my_allentities_path() -> String {
    allentities_path(DATA_MODEL_IRI)
}

/// The `@context` for the fixture: maps `myonto`, `sibling`, and system prefixes.
fn fixture_context() -> serde_json::Value {
    json!({
        "knora-api":  "http://api.knora.org/ontology/knora-api/v2#",
        "knora-base": "http://www.knora.org/ontology/knora-base#",
        "rdfs":       "http://www.w3.org/2000/01/rdf-schema#",
        "owl":        "http://www.w3.org/2002/07/owl#",
        "salsah-gui": "http://api.knora.org/ontology/salsah-gui/v2#",
        "myonto":     MY_NS,
        "sibling":    SIB_NS
    })
}

/// Build the full allentities fixture exercising all the edge cases.
///
/// Classes:
///
/// - `myonto:Letter`: inherits WrittenSource (same DM), inherits knora-base:Resource
///   (system/builtin), link hasSender → Person (same DM), link hasSenderValue (twin,
///   dropped), link hasSiblingLink → sibling:Thing (cross-DM target), link hasKnoraLink
///   → knora-api:Region (project field / builtin target; is_builtin=false, tdm=None).
/// - `myonto:WrittenSource`: superclass, also a resource class.
/// - `myonto:Person`: link target, resource class.
///
/// The `sibling:thingLink` property node is ABSENT from the graph, so the restriction
/// on Letter for that field is silently skipped (v1 limitation).
fn happy_fixture() -> serde_json::Value {
    json!({
        "@id": DATA_MODEL_IRI,
        "rdfs:label": "My ontology",
        "@graph": [
            // ── myonto:Letter class ─────────────────────────────────────────────
            {
                "@id": "myonto:Letter",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Letter",
                "rdfs:subClassOf": [
                    // project superclass (same DM)
                    { "@id": "myonto:WrittenSource" },
                    // system superclass (knora-base — must be is_builtin=true, tdm=None)
                    { "@id": "knora-base:Resource" },
                    // link restriction: myonto:hasSender → myonto:Person (same DM)
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "myonto:hasSender" },
                        "owl:minCardinality": 0
                    },
                    // twin — MUST be dropped
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "myonto:hasSenderValue" },
                        "owl:minCardinality": 0
                    },
                    // link restriction: myonto:hasSiblingLink → sibling:Thing (cross-DM target)
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "myonto:hasSiblingLink" },
                        "owl:minCardinality": 0
                    },
                    // link restriction: myonto:hasKnoraLink → knora-api:Region
                    // (project-defined field, target is system — is_builtin=false, tdm=None)
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "myonto:hasKnoraLink" },
                        "owl:minCardinality": 0
                    },
                    // absent cross-DM property restriction (node absent → v1 skip)
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "sibling:thingLink" },
                        "owl:minCardinality": 0
                    }
                ]
            },
            // ── myonto:WrittenSource class ──────────────────────────────────────
            {
                "@id": "myonto:WrittenSource",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Written Source",
                "rdfs:subClassOf": [
                    { "@id": "knora-base:Resource" }
                ]
            },
            // ── myonto:Person class ─────────────────────────────────────────────
            {
                "@id": "myonto:Person",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Person",
                "rdfs:subClassOf": [
                    { "@id": "knora-base:Resource" }
                ]
            },
            // ── property nodes ──────────────────────────────────────────────────
            // myonto:hasSender — link to myonto:Person (same DM)
            {
                "@id": "myonto:hasSender",
                "knora-api:isResourceProperty": true,
                "knora-api:isLinkProperty": true,
                "knora-api:objectType": { "@id": "myonto:Person" },
                "rdfs:label": "Has Sender"
            },
            // myonto:hasSenderValue — reification twin (MUST be dropped)
            {
                "@id": "myonto:hasSenderValue",
                "knora-api:isLinkValueProperty": true,
                "knora-api:objectType": { "@id": "knora-api:LinkValue" },
                "rdfs:label": "Has Sender (value)"
            },
            // myonto:hasSiblingLink — link to sibling:Thing (cross-DM target)
            {
                "@id": "myonto:hasSiblingLink",
                "knora-api:isResourceProperty": true,
                "knora-api:isLinkProperty": true,
                "knora-api:objectType": { "@id": "sibling:Thing" },
                "rdfs:label": "Has Sibling Link"
            },
            // myonto:hasKnoraLink — link to knora-api:Region (builtin target)
            {
                "@id": "myonto:hasKnoraLink",
                "knora-api:isResourceProperty": true,
                "knora-api:isLinkProperty": true,
                "knora-api:objectType": { "@id": "knora-api:Region" },
                "rdfs:label": "Has Knora Link"
            }
            // NOTE: sibling:thingLink property node is ABSENT — no node for it
            // → the restriction on Letter for sibling:thingLink will be skipped
        ],
        "@context": fixture_context()
    })
}

// ---------------------------------------------------------------------------
// Test 1: Happy path — full relation set
// ---------------------------------------------------------------------------

/// Happy path: the full fixture produces the expected `DataModelStructure`
/// with all the right relations, sort order, and edge annotations.
#[tokio::test]
async fn happy_path_data_model_structure() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(my_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(happy_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.data_model_structure(&uri, DATA_MODEL_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let structure: DataModelStructure = result.unwrap();

    // data_model name is derived from the IRI.
    assert_eq!(
        structure.data_model, "myonto",
        "data_model must be the local name derived from the IRI"
    );

    // ── Verify the hasSenderValue twin is NOT present ───────────────────────
    let twin_present = structure
        .relations
        .iter()
        .any(|r| r.field.as_deref() == Some("hasSenderValue"));
    assert!(
        !twin_present,
        "hasSenderValue (link-value twin) must be dropped from the relation list"
    );

    // ── Verify the sibling:thingLink absent-node skip ──────────────────────
    // The restriction `sibling:thingLink` on Letter should be skipped because
    // the property node is absent from the graph (v1 limitation).
    let absent_node_present = structure
        .relations
        .iter()
        .any(|r| r.field.as_deref() == Some("thingLink"));
    assert!(
        !absent_node_present,
        "sibling:thingLink restriction (node absent) must be skipped (v1 limitation)"
    );

    // ── Find and assert specific relations ────────────────────────────────────

    // 1. Letter → hasSender → Person (same-DM link)
    let has_sender = structure
        .relations
        .iter()
        .find(|r| r.source == "Letter" && r.field.as_deref() == Some("hasSender"))
        .expect("Letter:hasSender relation must be present");
    assert_eq!(has_sender.kind, RelationKind::Link);
    assert_eq!(has_sender.target, "Person");
    // Same-DM target: prefix is "myonto" — not a system prefix, so Some("myonto")
    assert_eq!(
        has_sender.target_data_model.as_deref(),
        Some("myonto"),
        "same-DM target: target_data_model must be Some(\"myonto\")"
    );
    // Field prefix is myonto (project) → not builtin
    assert!(
        !has_sender.is_builtin,
        "project link field hasSender must not be builtin"
    );

    // 2. Letter → hasSiblingLink → Thing (cross-DM target)
    let sib_link = structure
        .relations
        .iter()
        .find(|r| r.source == "Letter" && r.field.as_deref() == Some("hasSiblingLink"))
        .expect("Letter:hasSiblingLink relation must be present");
    assert_eq!(sib_link.kind, RelationKind::Link);
    assert_eq!(sib_link.target, "Thing");
    assert_eq!(
        sib_link.target_data_model.as_deref(),
        Some("sibling"),
        "cross-DM target: target_data_model must be Some(\"sibling\")"
    );
    assert!(
        !sib_link.is_builtin,
        "project link field hasSiblingLink must not be builtin"
    );

    // 3. Letter → hasKnoraLink → Region (project field → builtin target)
    let knora_link = structure
        .relations
        .iter()
        .find(|r| r.source == "Letter" && r.field.as_deref() == Some("hasKnoraLink"))
        .expect("Letter:hasKnoraLink relation must be present");
    assert_eq!(knora_link.kind, RelationKind::Link);
    assert_eq!(knora_link.target, "Region");
    // Target is system (knora-api) → target_data_model = None (invariant b)
    assert_eq!(
        knora_link.target_data_model, None,
        "system target knora-api:Region → target_data_model must be None"
    );
    // is_builtin is keyed off the FIELD's prefix (myonto → not system → false)
    assert!(
        !knora_link.is_builtin,
        "project-defined field hasKnoraLink pointing to builtin target must be is_builtin=false"
    );

    // 4. Letter inherits WrittenSource (project superclass, same DM)
    let inherits_written = structure
        .relations
        .iter()
        .find(|r| {
            r.source == "Letter" && r.kind == RelationKind::Inherits && r.target == "WrittenSource"
        })
        .expect("Letter inherits WrittenSource must be present");
    assert_eq!(
        inherits_written.field, None,
        "inherits relation must have field=None"
    );
    // WrittenSource prefix is myonto (project) — Some("myonto")
    assert_eq!(
        inherits_written.target_data_model.as_deref(),
        Some("myonto"),
        "project superclass WrittenSource: target_data_model must be Some(\"myonto\")"
    );
    assert!(
        !inherits_written.is_builtin,
        "inherits to project super WrittenSource must not be builtin"
    );

    // 5. Letter inherits knora-base:Resource (system superclass)
    let inherits_resource = structure
        .relations
        .iter()
        .find(|r| {
            r.source == "Letter" && r.kind == RelationKind::Inherits && r.target == "Resource"
        })
        .expect("Letter inherits knora-base:Resource must be present");
    assert_eq!(inherits_resource.field, None);
    // System superclass → target_data_model = None (invariant b)
    assert_eq!(
        inherits_resource.target_data_model, None,
        "system super knora-base:Resource → target_data_model must be None"
    );
    // is_builtin for Inherits: keyed off target prefix (knora-base → system → true)
    assert!(
        inherits_resource.is_builtin,
        "inherits to system super knora-base:Resource must be is_builtin=true"
    );

    // ── D6 sort order: (source, kind, field, target) ─────────────────────────
    // Link < Inherits within same source; None < Some for field (inherits have None).
    // Within Letter: links first (sorted by field name), then inherits.
    let letter_relations: Vec<&Relation> = structure
        .relations
        .iter()
        .filter(|r| r.source == "Letter")
        .collect();

    // All links must come before all inherits.
    let first_inherits = letter_relations
        .iter()
        .position(|r| r.kind == RelationKind::Inherits);
    let last_link = letter_relations
        .iter()
        .rposition(|r| r.kind == RelationKind::Link);
    if let (Some(fi), Some(ll)) = (first_inherits, last_link) {
        assert!(
            fi > ll,
            "all link relations must sort before inherits relations for the same source (D6)"
        );
    }

    // Within links for Letter: sorted by field name ascending.
    let letter_links: Vec<&str> = letter_relations
        .iter()
        .filter(|r| r.kind == RelationKind::Link)
        .map(|r| r.field.as_deref().unwrap_or(""))
        .collect();
    let mut sorted_links = letter_links.clone();
    sorted_links.sort();
    assert_eq!(
        letter_links, sorted_links,
        "link relations for Letter must be sorted by field name (D6)"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Bearer present — token forwarded to the allentities request
// ---------------------------------------------------------------------------

/// When `token = Some(TOKEN)` is passed, the request must carry
/// `Authorization: Bearer <TOKEN>`.
#[tokio::test]
async fn bearer_present_when_token_is_some() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(my_allentities_path()))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(happy_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.data_model_structure(&uri, DATA_MODEL_IRI, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "expected Ok when token is Some, got: {:?}",
        result
    );

    let received = server
        .received_requests()
        .await
        .expect("request recording should be enabled");
    assert_eq!(
        received.len(),
        1,
        "exactly one request must have been made (no sibling fetch)"
    );
    let auth = received[0]
        .headers
        .get("authorization")
        .expect("Authorization header must be present when token is Some");
    assert_eq!(
        auth.to_str().expect("header should be valid UTF-8"),
        format!("Bearer {TOKEN}"),
        "bearer token must match the value passed to data_model_structure"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Bearer absent — no Authorization header when token is None
// ---------------------------------------------------------------------------

/// When `token = None` is passed, NO `Authorization` header must be sent.
/// Guards against a future refactor accidentally sending an unconditional bearer.
#[tokio::test]
async fn bearer_absent_when_token_is_none() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(my_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(happy_fixture()))
        .mount(&server)
        .await;

    let uri = server.uri();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.data_model_structure(&uri, DATA_MODEL_IRI, None)
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
        "Authorization header must be ABSENT when token is None"
    );
}

// ---------------------------------------------------------------------------
// Test 4: Exactly one request — no sibling fetch
// ---------------------------------------------------------------------------

/// `data_model_structure` must make exactly ONE `allentities` request.
/// Cross-DM property nodes that are absent are silently skipped (v1 limitation).
/// This guards against the implementation accidentally doing a sibling fetch.
#[tokio::test]
async fn exactly_one_allentities_request_no_sibling_fetch() {
    let server = MockServer::start().await;

    // Mount with expect(1) — wiremock will fail the test if hit count != 1.
    Mock::given(method("GET"))
        .and(path(my_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(happy_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.data_model_structure(&uri, DATA_MODEL_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "expected Ok even when cross-DM property node is absent, got: {:?}",
        result
    );

    // wiremock asserts exactly 1 hit when server drops — this is belt-and-braces.
    let received = server
        .received_requests()
        .await
        .expect("request recording should be enabled");
    assert_eq!(
        received.len(),
        1,
        "must make exactly 1 request (no sibling fetch in data_model_structure)"
    );
}

// ---------------------------------------------------------------------------
// Test 5: Server error (500)
// ---------------------------------------------------------------------------

/// A 500 response from allentities returns `Err(Diagnostic::ServerError(_))`.
#[tokio::test]
async fn server_error_500_returns_server_error_diagnostic() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(my_allentities_path()))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.data_model_structure(&uri, DATA_MODEL_IRI, None)
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
// Test 6: Empty graph → empty relations
// ---------------------------------------------------------------------------

/// An ontology with no resource classes returns an empty `relations` vec.
#[tokio::test]
async fn empty_graph_yields_empty_relations() {
    let server = MockServer::start().await;

    let fixture = json!({
        "@id": DATA_MODEL_IRI,
        "rdfs:label": "Empty ontology",
        "@graph": [],
        "@context": fixture_context()
    });

    Mock::given(method("GET"))
        .and(path(my_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.data_model_structure(&uri, DATA_MODEL_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "expected Ok for empty graph, got: {:?}",
        result
    );
    let structure = result.unwrap();
    assert_eq!(
        structure.data_model, "myonto",
        "data_model must still be derived from IRI even for empty graph"
    );
    assert!(
        structure.relations.is_empty(),
        "empty graph must yield empty relations"
    );
}

// ---------------------------------------------------------------------------
// Test 7: D6 multi-source ordering
// ---------------------------------------------------------------------------

/// Verifies that the top-level sort by source puts sources in alphabetical order,
/// and within each source link relations come before inherits relations (D6).
#[tokio::test]
async fn d6_sort_order_multi_source() {
    let server = MockServer::start().await;

    // Two classes: Zebra (link + inherit) and Apple (link + inherit).
    // After D6 sort: Apple comes before Zebra at the source level.
    let fixture = json!({
        "@id": DATA_MODEL_IRI,
        "@graph": [
            {
                "@id": "myonto:Zebra",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Zebra",
                "rdfs:subClassOf": [
                    { "@id": "knora-base:Resource" },
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "myonto:zebraLink" },
                        "owl:minCardinality": 0
                    }
                ]
            },
            {
                "@id": "myonto:Apple",
                "@type": "owl:Class",
                "knora-api:isResourceClass": true,
                "rdfs:label": "Apple",
                "rdfs:subClassOf": [
                    { "@id": "knora-base:Resource" },
                    {
                        "@type": "owl:Restriction",
                        "owl:onProperty": { "@id": "myonto:appleLink" },
                        "owl:minCardinality": 0
                    }
                ]
            },
            {
                "@id": "myonto:zebraLink",
                "knora-api:isResourceProperty": true,
                "knora-api:isLinkProperty": true,
                "knora-api:objectType": { "@id": "myonto:Apple" }
            },
            {
                "@id": "myonto:appleLink",
                "knora-api:isResourceProperty": true,
                "knora-api:isLinkProperty": true,
                "knora-api:objectType": { "@id": "myonto:Zebra" }
            }
        ],
        "@context": fixture_context()
    });

    Mock::given(method("GET"))
        .and(path(my_allentities_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.data_model_structure(&uri, DATA_MODEL_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let structure = result.unwrap();

    // Verify global sort order: Apple-* comes before Zebra-*.
    let sources: Vec<&str> = structure
        .relations
        .iter()
        .map(|r| r.source.as_str())
        .collect();

    let apple_first = sources
        .iter()
        .position(|&s| s == "Apple")
        .unwrap_or(usize::MAX);
    let zebra_first = sources
        .iter()
        .position(|&s| s == "Zebra")
        .unwrap_or(usize::MAX);
    assert!(
        apple_first < zebra_first,
        "Apple relations must sort before Zebra relations (D6 source sort)"
    );

    // Within each source: link before inherits.
    for source in &["Apple", "Zebra"] {
        let src_rels: Vec<&Relation> = structure
            .relations
            .iter()
            .filter(|r| r.source.as_str() == *source)
            .collect();
        let first_inherit = src_rels
            .iter()
            .position(|r| r.kind == RelationKind::Inherits);
        let last_link = src_rels.iter().rposition(|r| r.kind == RelationKind::Link);
        if let (Some(fi), Some(ll)) = (first_inherit, last_link) {
            assert!(
                fi > ll,
                "for source {source}: link relations must come before inherits relations"
            );
        }
    }
}
