// Wiremock integration tests for `dsp vre resource describe --values` (Phase 8c).
//
// Tests exercise `HttpDspClient::describe_resource(server, iri, token, true)`
// end-to-end against a mock server. Three endpoints are mocked:
//
//   (a) GET /v2/resources/<enc-iri>?schema=complex  — the resource body with
//       several field values (text, int, link, list, still-image) and a project
//       ontology field (myonto:hasFoo) so a label fetch is triggered.
//
//   (b) GET /v2/ontologies/allentities/<enc-ontology-iri>  — returns a minimal
//       allentities doc with the property rdfs:label so field label resolves.
//
//   (c) GET /v2/node/<enc-node-iri>  — returns a list-node label.
//
// Assertions cover:
//   (1) Resource parses with values: Some(...)
//   (2) Link target label is populated from the embedded target (no extra fetch).
//   (3) Field label resolved from the ontology fetch.
//   (4) List-item label resolved from /v2/node fetch.
//   (5) Dedup — two fields from the same ontology trigger ONE ontology fetch.
//   (6) Degradation — ontology / node GET returns 404 → describe still succeeds,
//       field falls back to local name, vocabulary-item to node IRI.
//   (7) Standoff — text value carrying knora-api:textValueAsXml has XML stripped.
//   (8) SSRF host-pinning — /v2/node request hits the MOCK server host, not the
//       IRI's own host.
//
// `reqwest::blocking` + `std::thread::spawn` pattern matches resource_describe_http.rs:
// the blocking client lives and dies on its own OS thread (never on the tokio pool).
//
// ADR-0001 vocabulary guard: no DSP-API key names in domain-model assertions.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::model::ValueContent;
use serde_json::json;
use wiremock::matchers::{method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Test constants ─────────────────────────────────────────────────────────────

const TOKEN: &str = "test-values-token";

// A synthetic resource IRI that contains characters needing percent-encoding.
// (Same IRI used in 8b tests so we can re-use RESOURCE_PATH constant style.)
const RESOURCE_IRI: &str = "http://rdfh.ch/0803/--6Esp4SVnGG1DBzFvYErw";
const RESOURCE_PATH: &str =
    "/v2/resources/http%3A%2F%2Frdfh%2Ech%2F0803%2F%2D%2D6Esp4SVnGG1DBzFvYErw";

// The project ontology that houses `myonto:hasFoo`.
const MYONTO_IRI: &str = "http://api.dasch.swiss/ontology/0803/myonto/v2";
const MYONTO_NS: &str = "http://api.dasch.swiss/ontology/0803/myonto/v2#";
const MYONTO_ALLENTITIES_PATH: &str =
    "/v2/ontologies/allentities/http%3A%2F%2Fapi%2Edasch%2Eswiss%2Fontology%2F0803%2Fmyonto%2Fv2";

// A list-node IRI used in the fixture.
const NODE_IRI: &str = "http://rdfh.ch/lists/0803/genre-incunabula";
const NODE_IRI_ENC: &str = "http%3A%2F%2Frdfh%2Ech%2Flists%2F0803%2Fgenre%2Dincunabula";

// ── Fixture bodies ─────────────────────────────────────────────────────────────

/// Resource body with a mix of value types.
///
/// Value types included:
///   - `myonto:hasText`       → TextValue (plain, valueAsString)
///   - `myonto:hasStandoff`   → TextValue (formatted, textValueAsXml)
///   - `myonto:hasInt`        → IntValue (integer)
///   - `myonto:hasFoo`        → TextValue (triggers ontology label fetch — field label)
///   - `myonto:hasGenreValue` → LinkValue (embedded target with rdfs:label)
///   - `myonto:hasListItem`   → ListValue (list-node label via /v2/node)
///   - `knora-api:hasStillImageFileValue` → StillImageFileValue (built-in, no ontology fetch)
///
/// Denylisted keys that must NOT become fields:
///   - `knora-api:hasIncomingLinkValue`
///   - `knora-api:versionArkUrl` (has xsd:anyURI @type, not a *Value @type)
fn resource_body() -> serde_json::Value {
    json!({
        "@id": RESOURCE_IRI,
        "@type": "myonto:Page",
        "rdfs:label": "n6r",
        "knora-api:hasPermissions": "V knora-admin:UnknownUser",
        "knora-api:userHasPermission": "V",
        "knora-api:arkUrl": {
            "@value": "https://ark.stage.dasch.swiss/ark:/72163/1/0803/==6Esp",
            "@type": "xsd:anyURI"
        },
        // Should NOT appear as field — xsd:anyURI @type, not a knora-api *Value.
        "knora-api:versionArkUrl": {
            "@value": "https://ark.stage.dasch.swiss/ark:/72163/1/0803/==6Esp.20240310T150000000Z",
            "@type": "xsd:anyURI"
        },
        // Should NOT appear as field — on the denylist.
        "knora-api:hasIncomingLinkValue": {
            "@type": "knora-api:LinkValue",
            "knora-api:linkValueHasTargetIri": { "@id": "http://rdfh.ch/0803/other" }
        },
        // Plain text value.
        "myonto:hasText": {
            "@type": "knora-api:TextValue",
            "knora-api:valueAsString": "Page number n6r"
        },
        // Formatted text (standoff) — textValueAsXml should be stripped.
        "myonto:hasStandoff": {
            "@type": "knora-api:TextValue",
            "knora-api:textValueAsXml": "<text>Some <strong>bold</strong> text</text>"
        },
        // Integer value.
        "myonto:hasInt": {
            "@type": "knora-api:IntValue",
            "knora-api:intValueAsInt": 42
        },
        // Another text field in the same ontology (triggers dedup: same ontology,
        // should only cause ONE allentities fetch total).
        "myonto:hasFoo": {
            "@type": "knora-api:TextValue",
            "knora-api:valueAsString": "foo bar"
        },
        // Link value with embedded target (label free from the response, no extra fetch).
        "myonto:hasGenreValue": {
            "@type": "knora-api:LinkValue",
            "knora-api:linkValueHasTarget": {
                "@id": "http://rdfh.ch/0803/bookres123",
                "@type": "myonto:Book",
                "rdfs:label": "Incunabula Testbook"
            }
        },
        // List value — label to be fetched from /v2/node.
        "myonto:hasListItem": {
            "@type": "knora-api:ListValue",
            "knora-api:listValueAsListNode": {
                "@id": NODE_IRI
            }
        },
        // Still-image file value (built-in knora-api: key) — no ontology label fetch.
        "knora-api:hasStillImageFileValue": {
            "@type": "knora-api:StillImageFileValue",
            "knora-api:fileValueHasFilename": "n6r.jp2",
            "knora-api:fileValueAsUrl": {
                "@value": "https://iiif.dasch.swiss/0803/n6r.jp2/full/max/0/default.jpg",
                "@type": "xsd:anyURI"
            },
            "knora-api:stillImageFileValueHasDimX": 2048,
            "knora-api:stillImageFileValueHasDimY": 3072
        },
        "@context": {
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
            "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
            "myonto": MYONTO_NS,
            "xsd": "http://www.w3.org/2001/XMLSchema#"
        }
    })
}

/// Extended resource body for denylist coverage.
///
/// Adds two additional entries beyond `resource_body()`:
///   - `knora-api:hasStandoffLinkToValue` — a `*Value`-typed key on the denylist.
///   - `myonto:hasDeletedThing` — a value whose `@type` is `knora-api:DeletedValue`
///     (must be silently skipped; the whole field becomes empty and is dropped).
///
/// A real, non-denied field (`knora-api:hasStillImageFileValue`) is still present
/// so we can assert it DOES surface (non-vacuous positive check).
fn resource_body_extended_denylist() -> serde_json::Value {
    let mut body = resource_body();
    let obj = body.as_object_mut().expect("resource body must be object");
    // Add hasStandoffLinkToValue — denylist entry with a *Value @type.
    obj.insert(
        "knora-api:hasStandoffLinkToValue".to_string(),
        json!({
            "@type": "knora-api:LinkValue",
            "knora-api:linkValueHasTargetIri": { "@id": "http://rdfh.ch/0803/standoff-target" }
        }),
    );
    // Add a field whose sole value is a DeletedValue — must be dropped entirely.
    obj.insert(
        "myonto:hasDeletedThing".to_string(),
        json!({
            "@type": "knora-api:DeletedValue",
            "knora-api:valueHasComment": "deleted"
        }),
    );
    body
}

/// Allentities body for `myonto` ontology.
/// Contains property entries with rdfs:label so field labels resolve.
fn myonto_allentities_body() -> serde_json::Value {
    json!({
        "@id": MYONTO_IRI,
        "rdfs:label": "My Test Ontology",
        "@graph": [
            {
                "@id": "myonto:hasText",
                "rdfs:label": "Has Text",
                "knora-api:isResourceProperty": true
            },
            {
                "@id": "myonto:hasStandoff",
                "rdfs:label": "Has Standoff Text",
                "knora-api:isResourceProperty": true
            },
            {
                "@id": "myonto:hasInt",
                "rdfs:label": "Has Integer",
                "knora-api:isResourceProperty": true
            },
            {
                "@id": "myonto:hasFoo",
                "rdfs:label": "Has Foo Property",
                "knora-api:isResourceProperty": true
            },
            {
                "@id": "myonto:hasGenre",
                "rdfs:label": "Has Genre",
                "knora-api:isLinkProperty": true
            },
            {
                "@id": "myonto:hasListItem",
                "rdfs:label": "Has List Item",
                "knora-api:isResourceProperty": true
            }
        ],
        "@context": {
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
            "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
            "myonto": MYONTO_NS
        }
    })
}

/// List-node body for `NODE_IRI`.
fn node_body() -> serde_json::Value {
    json!({
        "@id": NODE_IRI,
        "@type": "knora-api:ListNode",
        "rdfs:label": "Incunabula Genre Node"
    })
}

// ── (1) + (2) + (3) + (4): happy-path multi-fetch ─────────────────────────────

/// Happy-path: resource fetch + ontology allentities + list-node.
///
/// Assertions:
///   (1) `values` is `Some` and non-empty.
///   (2) Link target label populated from the embedded target (free in complex schema).
///   (3) `myonto:hasFoo` field label resolves to "Has Foo Property" from the ontology.
///   (4) List-item label resolves to "Incunabula Genre Node" from /v2/node.
#[tokio::test]
async fn multi_fetch_happy_path() {
    let server = MockServer::start().await;

    // Mount the resource endpoint.
    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(resource_body()))
        .expect(1)
        .mount(&server)
        .await;

    // Mount the ontology allentities endpoint.
    Mock::given(method("GET"))
        .and(path(MYONTO_ALLENTITIES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(myonto_allentities_body()))
        .mount(&server)
        .await;

    // Mount the list-node endpoint (path is /v2/node/<enc-node-iri>).
    let node_path = format!("/v2/node/{NODE_IRI_ENC}");
    Mock::given(method("GET"))
        .and(path(&node_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(node_body()))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, Some(TOKEN), true)
    })
    .join()
    .expect("blocking thread should not panic");

    let detail = result.expect("multi-fetch happy-path must succeed");

    // (1) values is Some and non-empty.
    let fields = detail
        .values
        .expect("values must be Some when with_values=true");
    assert!(!fields.is_empty(), "fields must not be empty");

    // (2) Link target label: populated from embedded target, no extra fetch needed.
    let link_field = fields
        .iter()
        .find(|f| f.name == "hasGenre")
        .expect("must find hasGenre link field (Value-suffix stripped)");
    let link_val = &link_field.values[0];
    match &link_val.content {
        ValueContent::Link {
            target_label,
            target_iri,
        } => {
            assert_eq!(
                target_label.as_deref(),
                Some("Incunabula Testbook"),
                "link target label must be populated from embedded target; got: {target_label:?}"
            );
            assert_eq!(target_iri, "http://rdfh.ch/0803/bookres123");
        }
        other => panic!("expected Link value, got: {other:?}"),
    }

    // (3) Field label resolved from ontology fetch.
    let foo_field = fields
        .iter()
        .find(|f| f.name == "hasFoo")
        .expect("must find hasFoo field");
    assert_eq!(
        foo_field.label.as_deref(),
        Some("Has Foo Property"),
        "field label must resolve from ontology allentities; got: {:?}",
        foo_field.label
    );

    // (4) List-item label resolved from /v2/node.
    let list_field = fields
        .iter()
        .find(|f| f.name == "hasListItem")
        .expect("must find hasListItem field");
    let list_val = &list_field.values[0];
    match &list_val.content {
        ValueContent::VocabularyItem { label, node_iri } => {
            assert_eq!(
                label.as_deref(),
                Some("Incunabula Genre Node"),
                "vocabulary-item label must resolve from /v2/node; got: {label:?}"
            );
            assert_eq!(node_iri, NODE_IRI);
        }
        other => panic!("expected VocabularyItem value, got: {other:?}"),
    }
}

// ── (5) Dedup: same ontology → ONE allentities fetch ─────────────────────────

/// Two fields from the same project ontology (`myonto:hasText` and `myonto:hasFoo`)
/// must trigger exactly ONE `/v2/ontologies/allentities/<myonto>` request.
///
/// Uses `.expect(1)` on the allentities mock so wiremock asserts ≤1 hit.
/// If the impl fetches twice, wiremock will report an unexpected call.
#[tokio::test]
async fn dedup_same_ontology_one_fetch() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(resource_body()))
        .expect(1)
        .mount(&server)
        .await;

    // Expect exactly 1 ontology fetch despite multiple fields from myonto.
    Mock::given(method("GET"))
        .and(path(MYONTO_ALLENTITIES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(myonto_allentities_body()))
        .expect(1) // The load-bearing dedup assertion.
        .mount(&server)
        .await;

    // Provide a node endpoint so the list-node fetch doesn't fail.
    let node_path = format!("/v2/node/{NODE_IRI_ENC}");
    Mock::given(method("GET"))
        .and(path(&node_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(node_body()))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, true)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "dedup test must succeed; got: {:?}", result);
    // wiremock verifies .expect(1) on the allentities mock when the MockServer drops.
}

// ── (6) Degradation: ontology 404 ─────────────────────────────────────────────

/// When the ontology allentities endpoint returns 404, `describe_resource` must
/// still SUCCEED and field labels must fall back to the local name (not error).
#[tokio::test]
async fn degradation_ontology_404_succeeds() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(resource_body()))
        .mount(&server)
        .await;

    // Ontology fetch returns 404.
    Mock::given(method("GET"))
        .and(path(MYONTO_ALLENTITIES_PATH))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    // Node fetch still succeeds.
    let node_path = format!("/v2/node/{NODE_IRI_ENC}");
    Mock::given(method("GET"))
        .and(path(&node_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(node_body()))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, true)
    })
    .join()
    .expect("blocking thread should not panic");

    let detail = result.expect("describe_resource must succeed even when ontology 404s");
    let fields = detail.values.expect("values must be Some");

    // Field labels should be None (degraded to no label) when ontology fetch failed.
    let foo_field = fields
        .iter()
        .find(|f| f.name == "hasFoo")
        .expect("hasFoo field must be present even when ontology 404s");
    assert!(
        foo_field.label.is_none(),
        "field label must be None (degraded) when ontology 404; got: {:?}",
        foo_field.label
    );

    // List-item label should still resolve (node fetch succeeded).
    let list_field = fields
        .iter()
        .find(|f| f.name == "hasListItem")
        .expect("hasListItem field must be present even when ontology 404s");
    let list_val = list_field
        .values
        .first()
        .expect("hasListItem must have at least one value");
    match &list_val.content {
        ValueContent::VocabularyItem { label, .. } => {
            assert_eq!(
                label.as_deref(),
                Some("Incunabula Genre Node"),
                "vocabulary-item label must still resolve when only ontology fetch fails; got: {label:?}"
            );
        }
        other => panic!("expected VocabularyItem value in hasListItem field, got: {other:?}"),
    }
}

/// When the /v2/node endpoint returns 404, `describe_resource` must still SUCCEED
/// and the vocabulary-item must fall back to the node IRI (not error).
#[tokio::test]
async fn degradation_node_404_succeeds() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(resource_body()))
        .mount(&server)
        .await;

    // Ontology fetch succeeds.
    Mock::given(method("GET"))
        .and(path(MYONTO_ALLENTITIES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(myonto_allentities_body()))
        .mount(&server)
        .await;

    // Node fetch returns 404.
    let node_path = format!("/v2/node/{NODE_IRI_ENC}");
    Mock::given(method("GET"))
        .and(path(&node_path))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, true)
    })
    .join()
    .expect("blocking thread should not panic");

    let detail = result.expect("describe_resource must succeed even when node 404s");
    let fields = detail.values.expect("values must be Some");

    // List-item label must be None (degraded to node IRI fallback).
    let list_field = fields
        .iter()
        .find(|f| f.name == "hasListItem")
        .expect("hasListItem field must be present even when /v2/node 404s");
    let list_val = list_field
        .values
        .first()
        .expect("hasListItem must have at least one value");
    match &list_val.content {
        ValueContent::VocabularyItem { label, node_iri } => {
            assert!(
                label.is_none(),
                "vocabulary-item label must be None when /v2/node returns 404; got: {label:?}"
            );
            assert_eq!(
                node_iri, NODE_IRI,
                "vocabulary-item node_iri must still be the IRI when label fetch fails"
            );
        }
        other => panic!("expected VocabularyItem value in hasListItem field, got: {other:?}"),
    }
}

// ── (7) Standoff text: textValueAsXml is stripped ─────────────────────────────

/// A TextValue with `textValueAsXml` must have the XML tags stripped.
/// The raw XML `<text>Some <strong>bold</strong> text</text>` should yield plain text.
#[tokio::test]
async fn standoff_text_xml_is_stripped() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(resource_body()))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(MYONTO_ALLENTITIES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(myonto_allentities_body()))
        .mount(&server)
        .await;

    let node_path = format!("/v2/node/{NODE_IRI_ENC}");
    Mock::given(method("GET"))
        .and(path(&node_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(node_body()))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let detail = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, true)
    })
    .join()
    .expect("blocking thread should not panic")
    .expect("standoff test must succeed");

    let fields = detail.values.expect("values must be Some");
    let standoff_field = fields
        .iter()
        .find(|f| f.name == "hasStandoff")
        .expect("must find hasStandoff field");
    match &standoff_field.values[0].content {
        ValueContent::Text(s) => {
            assert!(
                !s.contains('<'),
                "standoff text must have XML tags stripped; got: {s:?}"
            );
            assert!(
                !s.contains("strong"),
                "standoff text must have 'strong' tag stripped; got: {s:?}"
            );
            assert!(
                s.contains("bold") || s.contains("text"),
                "standoff text must contain readable text content; got: {s:?}"
            );
        }
        other => panic!("expected Text value, got: {other:?}"),
    }
}

// ── (8) SSRF host-pinning: /v2/node hits the MOCK server host ─────────────────

/// The `/v2/node/<enc-iri>` request must go to the MOCK SERVER's host,
/// NOT to the host encoded in the node IRI (`rdfh.ch`).
///
/// If the implementation incorrectly used the node IRI's host as the request
/// target, it would bypass the mock server and fail (connection refused).
/// Passing here confirms SSRF host-pinning is correct.
#[tokio::test]
async fn ssrf_node_request_hits_mock_server() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(resource_body()))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(MYONTO_ALLENTITIES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(myonto_allentities_body()))
        .mount(&server)
        .await;

    // The path pattern for /v2/node confirms the mock server (not rdfh.ch) is hit.
    // If the SSRF guard is broken, the request goes to rdfh.ch and never reaches
    // this mock — the MockServer assertion that the endpoint was called would fail.
    let node_path = format!("/v2/node/{NODE_IRI_ENC}");
    Mock::given(method("GET"))
        .and(path(&node_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(node_body()))
        .expect(1) // SSRF guard: must see exactly 1 hit, on the mock server host.
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, true)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "SSRF test must succeed; got: {:?}", result);
    // wiremock verifies .expect(1) on the node mock at MockServer drop — confirms
    // the node request reached the mock (not rdfh.ch's host).
}

// ── (b) Denylist: hasIncomingLinkValue and versionArkUrl do NOT appear as fields

/// `knora-api:hasIncomingLinkValue` (on the denylist) and `knora-api:versionArkUrl`
/// (xsd:anyURI @type, not a *Value) must NOT appear as user fields in the parsed result.
#[tokio::test]
async fn denylist_keys_not_surfaced_as_fields() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(resource_body()))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(MYONTO_ALLENTITIES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(myonto_allentities_body()))
        .mount(&server)
        .await;

    let node_path = format!("/v2/node/{NODE_IRI_ENC}");
    Mock::given(method("GET"))
        .and(path(&node_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(node_body()))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let detail = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, true)
    })
    .join()
    .expect("blocking thread should not panic")
    .expect("denylist test must succeed");

    let fields = detail.values.expect("values must be Some");

    // hasIncomingLinkValue must NOT appear.
    let incoming = fields.iter().find(|f| {
        f.name.to_lowercase().contains("incominglinkvalue") || f.name == "hasIncomingLinkValue"
    });
    assert!(
        incoming.is_none(),
        "hasIncomingLinkValue must NOT appear as a user field; fields: {:?}",
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
    );

    // versionArkUrl must NOT appear (xsd:anyURI @type).
    let version_ark = fields.iter().find(|f| {
        f.name.to_lowercase().contains("version") || f.name.to_lowercase().contains("arkurl")
    });
    assert!(
        version_ark.is_none(),
        "versionArkUrl must NOT appear as a user field; fields: {:?}",
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
    );
}

// ── (W2) Extended denylist coverage ──────────────────────────────────────────

/// Extended denylist: `knora-api:hasStandoffLinkToValue` (a `*Value`-typed key)
/// AND a value whose `@type` is `knora-api:DeletedValue` must NOT surface as
/// user fields.  A real field (`hasStillImageFileValue`) must still appear
/// (non-vacuous positive check).
#[tokio::test]
async fn denylist_extended_standoff_link_and_deleted_value() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(resource_body_extended_denylist()))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(MYONTO_ALLENTITIES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(myonto_allentities_body()))
        .mount(&server)
        .await;

    let node_path = format!("/v2/node/{NODE_IRI_ENC}");
    Mock::given(method("GET"))
        .and(path(&node_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(node_body()))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let detail = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, true)
    })
    .join()
    .expect("blocking thread should not panic")
    .expect("extended denylist test must succeed");

    let fields = detail.values.expect("values must be Some");
    let field_names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();

    // hasStandoffLinkToValue must NOT appear (on the explicit denylist).
    let standoff = fields.iter().find(|f| {
        f.name.to_lowercase().contains("standofflinkto") || f.name == "hasStandoffLinkToValue"
    });
    assert!(
        standoff.is_none(),
        "hasStandoffLinkToValue must NOT appear as a user field; fields: {field_names:?}"
    );

    // DeletedValue field must NOT appear (its only value is deleted → empty → dropped).
    let deleted = fields
        .iter()
        .find(|f| f.name.to_lowercase().contains("deleted"));
    assert!(
        deleted.is_none(),
        "DeletedValue field must NOT appear as a user field; fields: {field_names:?}"
    );

    // hasIncomingLinkValue must NOT appear (existing denylist, still guarded).
    let incoming = fields.iter().find(|f| {
        f.name.to_lowercase().contains("incominglinkvalue") || f.name == "hasIncomingLinkValue"
    });
    assert!(
        incoming.is_none(),
        "hasIncomingLinkValue must NOT appear as a user field; fields: {field_names:?}"
    );

    // versionArkUrl must NOT appear (xsd:anyURI @type, existing guard).
    let version_ark = fields.iter().find(|f| {
        f.name.to_lowercase().contains("version") || f.name.to_lowercase().contains("arkurl")
    });
    assert!(
        version_ark.is_none(),
        "versionArkUrl must NOT appear as a user field; fields: {field_names:?}"
    );

    // hasStillImageFileValue MUST appear (non-vacuous positive check).
    let file_field = fields.iter().find(|f| f.name == "hasStillImageFileValue");
    assert!(
        file_field.is_some(),
        "hasStillImageFileValue must appear as a real field; fields: {field_names:?}"
    );
}

// ── with_values=false: still only one request ────────────────────────────────

/// When `with_values=false`, only the resource fetch is made — NO ontology or
/// node fetches.
///
/// `.expect(0)` on the allentities mock asserts it is never called.
#[tokio::test]
async fn with_values_false_no_extra_fetches() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(resource_body()))
        .expect(1)
        .mount(&server)
        .await;

    // The allentities endpoint must NOT be called when with_values=false.
    Mock::given(method("GET"))
        .and(path_regex(r"/v2/ontologies/allentities/.*"))
        .respond_with(ResponseTemplate::new(500).set_body_string("should not be called"))
        .expect(0) // Asserts zero hits.
        .mount(&server)
        .await;

    // /v2/node must NOT be called when with_values=false.
    Mock::given(method("GET"))
        .and(path_regex(r"/v2/node/.*"))
        .respond_with(ResponseTemplate::new(500).set_body_string("should not be called"))
        .expect(0) // Asserts zero hits.
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false) // with_values=false
    })
    .join()
    .expect("blocking thread should not panic");

    let detail = result.expect("with_values=false must succeed");
    assert!(
        detail.values.is_none(),
        "values must be None when with_values=false; got: {:?}",
        detail.values
    );
    // wiremock checks .expect(0) on the allentities + node mocks at drop.
}

// ── (9) Per-value comment (`knora-api:valueHasComment`) ──────────────────────

/// A resource body whose single field is a `knora-api:`-prefixed scalar TextValue
/// (a built-in field, so no ontology allentities fetch is triggered) carrying a
/// sibling `knora-api:valueHasComment` key.
///
/// Deliberately minimal/standalone (approach (b)): its own fixture, its own
/// resource-endpoint mock — no ontology or node mocks needed, since a
/// `knora-api:`-prefixed field key is a system prefix and skips the label
/// fetch (see `parse_resource_values` — `is_system_prefix` check).
fn resource_body_with_commented_value() -> serde_json::Value {
    json!({
        "@id": RESOURCE_IRI,
        "@type": "myonto:Page",
        "rdfs:label": "n6r",
        "knora-api:hasComment": {
            "@type": "knora-api:TextValue",
            "knora-api:valueAsString": "Hello world",
            "knora-api:valueHasComment": "reading uncertain"
        }
    })
}

/// End-to-end: `describe_resource` → `ResourceDetail.values` → the specific
/// field's `Value.comment` must carry the server-supplied
/// `knora-api:valueHasComment` text through the full HTTP parse path.
#[tokio::test]
async fn value_with_comment_is_parsed_end_to_end() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(resource_body_with_commented_value()),
        )
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let detail = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, true)
    })
    .join()
    .expect("blocking thread should not panic")
    .expect("describe_resource with a commented value must succeed");

    let fields = detail
        .values
        .expect("values must be Some when with_values=true");
    let commented_field = fields
        .iter()
        .find(|f| f.name == "hasComment")
        .expect("must find hasComment field");
    let value = commented_field
        .values
        .first()
        .expect("hasComment must have at least one value");

    assert_eq!(
        value.comment.as_deref(),
        Some("reading uncertain"),
        "value.comment must carry knora-api:valueHasComment end-to-end; got: {:?}",
        value.comment
    );
    assert_eq!(
        value.content,
        ValueContent::Text("Hello world".into()),
        "commented value's content must still parse normally"
    );
}
