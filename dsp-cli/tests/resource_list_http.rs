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
// All fixture IRIs use real DaSCH project shapes (incunabula / BEOL).
// The endpoint requires the `x-knora-accept-project` header to scope
// results to a specific project and `resourceClass` to select the type.
//
// ADR-0001 vocabulary guard: no `ontology`/`export`/`class`/`property` as
// user-visible output. IRIs on the wire are boundary-translated before
// leaving the client.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use serde_json::json;
use wiremock::matchers::{header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// A clearly synthetic token — never a real JWT.
const TOKEN: &str = "test-token";

// The project IRI used across all tests (incunabula project — a well-known
// DaSCH research project with stable data for testing).
const PROJECT_IRI: &str = "http://rdfh.ch/projects/0803";

// The resource class IRI for incunabula:Page — a real resource type in the
// incunabula data-model. Used across all tests.
const RESOURCE_CLASS_IRI: &str = "http://api.dasch.swiss/ontology/0803/incunabula/v2#page";

// The endpoint path for the resource list.
const RESOURCES_PATH: &str = "/v2/resources";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Mount a mock that returns `body` as a 200 JSON response for GET /v2/resources.
async fn mount_200(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

/// Build a minimal resource node suitable for `@graph` inclusion.
fn resource_node(
    id: &str,
    label: &str,
    type_iri: &str,
    ark_url: Option<&str>,
    creation_date: Option<&str>,
) -> serde_json::Value {
    let mut node = json!({
        "@id": id,
        "@type": [type_iri, "http://api.knora.org/ontology/knora-api/v2#Resource"],
        "rdfs:label": label,
    });
    if let Some(ark) = ark_url {
        node["knora-api:arkUrl"] = json!({ "@value": ark });
    }
    if let Some(cd) = creation_date {
        node["knora-api:creationDate"] = json!({ "@value": cd });
    }
    node
}

// ---------------------------------------------------------------------------
// (a) Paged progression: page=0 → mayHaveMoreResults:true, page=1 → false
// ---------------------------------------------------------------------------

/// A two-page progression: the first call returns `may_have_more_results: true`
/// (page 0), the second call returns `may_have_more_results: false` (page 1).
/// This guards the `--all` loop termination: the client must translate
/// `knora-api:mayHaveMoreResults` correctly for both `true` and `false`.
#[tokio::test]
async fn paged_progression_page0_more_true_page1_false() {
    let server = MockServer::start().await;

    // Page 0: two resources, more results available.
    let body_page0 = json!({
        "@graph": [
            resource_node(
                "http://rdfh.ch/0803/resource-0001",
                "Page 1r",
                RESOURCE_CLASS_IRI,
                Some("http://ark.dasch.swiss/ark:/72163/1/0803/resource-0001"),
                Some("2023-01-01T00:00:00Z"),
            ),
            resource_node(
                "http://rdfh.ch/0803/resource-0002",
                "Page 1v",
                RESOURCE_CLASS_IRI,
                None,
                Some("2023-01-02T00:00:00Z"),
            ),
        ],
        "knora-api:mayHaveMoreResults": true,
    });

    // Page 1: one resource, no more results.
    let body_page1 = json!({
        "@graph": [
            resource_node(
                "http://rdfh.ch/0803/resource-0003",
                "Page 2r",
                RESOURCE_CLASS_IRI,
                Some("http://ark.dasch.swiss/ark:/72163/1/0803/resource-0003"),
                Some("2023-01-03T00:00:00Z"),
            ),
        ],
        "knora-api:mayHaveMoreResults": false,
    });

    // Mount page=0 mock (higher priority, exact query param match).
    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .and(query_param("page", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body_page0))
        .expect(1)
        .mount(&server)
        .await;

    // Mount page=1 mock.
    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body_page1))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let project_iri = PROJECT_IRI.to_string();
    let class_iri = RESOURCE_CLASS_IRI.to_string();

    // Page 0: should have may_have_more_results = true.
    let result_p0 = std::thread::spawn({
        let uri = uri.clone();
        let project_iri = project_iri.clone();
        let class_iri = class_iri.clone();
        move || {
            let client = HttpDspClient::new().expect("client construction should not fail");
            client.list_resources(&uri, &project_iri, &class_iri, None, 0, None)
        }
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result_p0.is_ok(),
        "page 0 must succeed, got: {:?}",
        result_p0
    );
    let page0 = result_p0.unwrap();
    assert_eq!(page0.resources.len(), 2, "page 0 must have 2 resources");
    assert!(
        page0.may_have_more_results,
        "page 0 must report may_have_more_results: true"
    );
    assert_eq!(page0.resources[0].label, "Page 1r");
    assert_eq!(page0.resources[0].iri, "http://rdfh.ch/0803/resource-0001");
    assert!(
        page0.resources[0].ark_url.is_some(),
        "page 0, item 0: ark_url must be Some"
    );
    assert!(
        page0.resources[0].creation_date.is_some(),
        "page 0, item 0: creation_date must be Some"
    );
    assert!(
        page0.resources[1].ark_url.is_none(),
        "page 0, item 1: ark_url absent in fixture must be None"
    );
    // resource_type must be derived via local_name() from @type[0] in the @graph form.
    // Guards that a bug reading @type from the @graph array form is caught, not only
    // the single-result form (tested separately in single_result_no_graph_parses_to_one_resource).
    assert_eq!(
        page0.resources[0].resource_type, "page",
        "@graph form: resource_type must be the local name of @type[0]; \
         got: {:?}",
        page0.resources[0].resource_type
    );
    assert_eq!(
        page0.resources[1].resource_type, "page",
        "@graph form: resource_type for item[1] must also be correct"
    );

    // Page 1: should have may_have_more_results = false.
    let result_p1 = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, &project_iri, &class_iri, None, 1, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result_p1.is_ok(),
        "page 1 must succeed, got: {:?}",
        result_p1
    );
    let page1 = result_p1.unwrap();
    assert_eq!(page1.resources.len(), 1, "page 1 must have 1 resource");
    assert!(
        !page1.may_have_more_results,
        "page 1 must report may_have_more_results: false"
    );
}

// ---------------------------------------------------------------------------
// (b) Empty-final-page variant: page=1 returns no @graph/empty with false
// ---------------------------------------------------------------------------

/// An empty final page: page 1 has no `@graph` and no `@id`, and reports
/// `may_have_more_results: false`. This exercises the empty-final-page variant
/// from D5: the `--all` loop must accept an empty page and stop cleanly (zero
/// resources accumulated from that page, termination on `false`).
#[tokio::test]
async fn empty_final_page_returns_zero_resources_with_false() {
    let server = MockServer::start().await;

    // Page 0: real content, server says more.
    let body_page0 = json!({
        "@graph": [
            resource_node(
                "http://rdfh.ch/0803/resource-0010",
                "Folio Av",
                RESOURCE_CLASS_IRI,
                None,
                None,
            ),
        ],
        "knora-api:mayHaveMoreResults": true,
    });

    // Page 1: no @graph, no @id, may_have_more_results = false.
    // This is the "total = exact multiple of page size" case described in D5.
    let body_page1 = json!({
        "knora-api:mayHaveMoreResults": false,
    });

    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .and(query_param("page", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body_page0))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body_page1))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let project_iri = PROJECT_IRI.to_string();
    let class_iri = RESOURCE_CLASS_IRI.to_string();

    // Page 0: one resource, more=true.
    let result_p0 = std::thread::spawn({
        let uri = uri.clone();
        let project_iri = project_iri.clone();
        let class_iri = class_iri.clone();
        move || {
            let client = HttpDspClient::new().expect("client construction should not fail");
            client.list_resources(&uri, &project_iri, &class_iri, None, 0, None)
        }
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result_p0.is_ok(),
        "page 0 must succeed, got: {:?}",
        result_p0
    );
    let page0 = result_p0.unwrap();
    assert_eq!(page0.resources.len(), 1);
    assert!(page0.may_have_more_results, "page 0 must say more=true");

    // Page 1: zero resources, more=false.
    let result_p1 = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, &project_iri, &class_iri, None, 1, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result_p1.is_ok(),
        "empty page 1 must succeed (not error), got: {:?}",
        result_p1
    );
    let page1 = result_p1.unwrap();
    assert_eq!(
        page1.resources.len(),
        0,
        "empty final page must yield zero resources"
    );
    assert!(
        !page1.may_have_more_results,
        "empty final page must report may_have_more_results: false"
    );
}

// ---------------------------------------------------------------------------
// (c) `resourceClass` query param is sent
// ---------------------------------------------------------------------------

/// The `resourceClass` query parameter must be present in every request and
/// carry the class IRI. A wiremock mock gated on the exact query param value
/// guards that the HTTP client sends it — a regression that drops `.query()`
/// would send the request without the param, which would not match the
/// `query_param` mock and return 404 (failing the test).
#[tokio::test]
async fn resource_class_query_param_is_sent() {
    let server = MockServer::start().await;

    // Mount a mock that ONLY matches when resourceClass equals the expected IRI.
    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .and(query_param("resourceClass", RESOURCE_CLASS_IRI))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "request must succeed when resourceClass param is sent, got: {:?}",
        result
    );

    // Also verify via recorded requests.
    let received = server
        .received_requests()
        .await
        .expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");

    // The query string must contain the encoded resourceClass IRI.
    let query = received[0].url.query().unwrap_or("");
    assert!(
        query.contains("resourceClass"),
        "query string must contain 'resourceClass'; got: {query:?}"
    );
}

// ---------------------------------------------------------------------------
// (d) `x-knora-accept-project` header is sent with the project IRI
// ---------------------------------------------------------------------------

/// The `x-knora-accept-project` header must be sent with the project IRI as
/// its value. This guards that the HTTP client sets this header — without it
/// the DSP-API would not scope the response to the project.
#[tokio::test]
async fn x_knora_accept_project_header_is_sent() {
    let server = MockServer::start().await;

    // Mount a mock that ONLY matches when the header is present with the correct value.
    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .and(header("x-knora-accept-project", PROJECT_IRI))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "request must succeed when x-knora-accept-project header is sent, got: {:?}",
        result
    );

    // Verify via recorded requests.
    let received = server
        .received_requests()
        .await
        .expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    let project_header = received[0]
        .headers
        .get("x-knora-accept-project")
        .expect("x-knora-accept-project header must be present");
    assert_eq!(
        project_header.to_str().expect("header must be valid UTF-8"),
        PROJECT_IRI,
        "x-knora-accept-project header value must equal the project IRI"
    );
}

/// Negative check: when a DIFFERENT project IRI is passed, the mock gated on
/// `PROJECT_IRI` must receive zero hits — confirming the header-value match is
/// tight and the client does not hardcode a different IRI.
///
/// A fallback mock (no header requirement) captures the actual request so it
/// does not produce a 404; the `expect(0)` on the PROJECT_IRI-gated mock is
/// validated by wiremock at server drop.
#[tokio::test]
async fn x_knora_accept_project_different_iri_does_not_match_project_iri_mock() {
    let server = MockServer::start().await;

    // A DIFFERENT project IRI — this must NOT match the PROJECT_IRI-gated mock.
    const OTHER_PROJECT_IRI: &str = "http://rdfh.ch/projects/0801";

    // Mock gated on PROJECT_IRI: must receive ZERO hits when we pass OTHER_PROJECT_IRI.
    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .and(header("x-knora-accept-project", PROJECT_IRI))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(0) // Must NOT be called — wrong header value.
        .mount(&server)
        .await;

    // Fallback mock (no header requirement) — captures the actual request.
    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        // Pass a different project IRI — the header value sent will be OTHER_PROJECT_IRI.
        client.list_resources(&uri, OTHER_PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "list_resources with a different project IRI must succeed via fallback mock, got: {:?}",
        result
    );
    // The `expect(0)` on the PROJECT_IRI-gated mock is verified at server drop.
}

// ---------------------------------------------------------------------------
// (e) Zero-result response (no @graph, no @id)
// ---------------------------------------------------------------------------

/// A response with neither `@graph` nor `@id` must yield an empty
/// `ResourcePage` (zero resources, `may_have_more_results: false`), not an
/// error. This is the well-known "no resources of that type" case.
#[tokio::test]
async fn zero_result_response_returns_empty_resource_page() {
    let server = MockServer::start().await;

    mount_200(&server, json!({})).await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "zero-result response must yield Ok, got: {:?}",
        result
    );
    let page = result.unwrap();
    assert!(
        page.resources.is_empty(),
        "zero-result response must yield empty resources"
    );
    assert!(
        !page.may_have_more_results,
        "zero-result response must yield may_have_more_results: false"
    );
}

/// Explicit zero-result with `may_have_more_results: false` in the body.
#[tokio::test]
async fn zero_result_with_explicit_false_flag() {
    let server = MockServer::start().await;

    mount_200(&server, json!({ "knora-api:mayHaveMoreResults": false })).await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let page = result.unwrap();
    assert!(page.resources.is_empty());
    assert!(!page.may_have_more_results);
}

// ---------------------------------------------------------------------------
// (f) Single-result form (no @graph but has @id) parses to exactly one resource
// ---------------------------------------------------------------------------

/// The JSON-LD single-result form: the server returns the resource at the
/// top level (no `@graph` wrapper) when there is exactly one matching resource.
/// This is a critical edge case — it must NOT be misread as empty (which would
/// happen if the code only checks `@graph` presence).
///
/// Assertion: exactly one `ResourceSummary` is returned, with the correct IRI,
/// label, resource_type (derived via `local_name`), and optional envelope fields.
#[tokio::test]
async fn single_result_no_graph_parses_to_one_resource() {
    let server = MockServer::start().await;

    // Top-level @id (no @graph) — single-result form from the DSP-API.
    let body = json!({
        "@id": "http://rdfh.ch/0803/incunabula-res-solo",
        "@type": [
            RESOURCE_CLASS_IRI,
            "http://api.knora.org/ontology/knora-api/v2#Resource",
        ],
        "rdfs:label": "Folio Solo",
        "knora-api:arkUrl": {
            "@value": "http://ark.dasch.swiss/ark:/72163/1/0803/solo"
        },
        "knora-api:creationDate": {
            "@value": "2024-03-15T10:00:00Z"
        },
        "knora-api:mayHaveMoreResults": false,
    });

    mount_200(&server, body).await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "single-result form must yield Ok, got: {:?}",
        result
    );
    let page = result.unwrap();

    assert_eq!(
        page.resources.len(),
        1,
        "single-result form (no @graph, @id present) must yield exactly 1 resource"
    );

    let r = &page.resources[0];
    assert_eq!(
        r.iri, "http://rdfh.ch/0803/incunabula-res-solo",
        "resource IRI must match top-level @id"
    );
    assert_eq!(
        r.label, "Folio Solo",
        "resource label must match rdfs:label"
    );

    // resource_type is derived via local_name() from the @type array's first element.
    assert_eq!(
        r.resource_type, "page",
        "resource_type must be derived from the local name of @type[0]; \
         got: {:?}",
        r.resource_type
    );

    assert_eq!(
        r.ark_url.as_deref(),
        Some("http://ark.dasch.swiss/ark:/72163/1/0803/solo"),
        "ark_url must be extracted from the knora-api:arkUrl @value field"
    );
    assert_eq!(
        r.creation_date.as_deref(),
        Some("2024-03-15T10:00:00Z"),
        "creation_date must be extracted from the knora-api:creationDate @value field"
    );

    assert!(
        !page.may_have_more_results,
        "single-result must report may_have_more_results: false"
    );
}

/// Single-result without optional envelope fields (no ark_url, no creation_date).
/// Guards the `Option` fallback — both must be `None`.
#[tokio::test]
async fn single_result_without_optional_fields_yields_none() {
    let server = MockServer::start().await;

    let body = json!({
        "@id": "http://rdfh.ch/0803/no-optional-fields",
        "@type": RESOURCE_CLASS_IRI,
        "rdfs:label": "Minimal Resource",
        "knora-api:mayHaveMoreResults": false,
    });

    mount_200(&server, body).await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "single-result without optionals must yield Ok, got: {:?}",
        result
    );
    let page = result.unwrap();
    assert_eq!(page.resources.len(), 1);
    let r = &page.resources[0];
    assert!(
        r.ark_url.is_none(),
        "ark_url must be None when absent from response"
    );
    assert!(
        r.creation_date.is_none(),
        "creation_date must be None when absent from response"
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
        .and(path(RESOURCES_PATH))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, Some(TOKEN))
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
        "bearer token must match the value passed to list_resources"
    );
}

// ---------------------------------------------------------------------------
// Bearer absent
// ---------------------------------------------------------------------------

/// When `token = None` is passed, NO `Authorization` header must be sent.
#[tokio::test]
async fn bearer_absent_when_token_is_none() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
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

/// Complementary bearer-absent check via `expect(0)` on a bearer-gated mock.
#[tokio::test]
async fn no_token_does_not_match_bearer_gated_mock() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(0) // Must NOT be called when token is None.
        .mount(&server)
        .await;

    // Fallback (no auth requirement).
    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "list_resources(None) must succeed via fallback mock, got: {:?}",
        result
    );
    // The `expect(0)` on the bearer-gated mock is verified at drop.
}

// ---------------------------------------------------------------------------
// 500 → ServerError
// ---------------------------------------------------------------------------

/// A 500 response maps to `Diagnostic::ServerError`.
#[tokio::test]
async fn server_error_500_returns_server_error_diagnostic() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
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
        .and(path(RESOURCES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string("this is not json at all"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
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
// Network failure → Diagnostic::Network
// ---------------------------------------------------------------------------

/// A connection failure (nothing listening) must map to `Diagnostic::Network`.
#[tokio::test]
async fn list_resources_connection_refused_returns_network() {
    let uri = "http://127.0.0.1:1".to_string();

    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err when server is unreachable");
    let err = result.unwrap_err();
    assert!(
        matches!(err, Diagnostic::Network(_)),
        "expected Diagnostic::Network for connection failure, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// `orderByProperty` query param: present when order_by=Some, absent when None
// ---------------------------------------------------------------------------

/// When `order_by = Some(iri)` is passed, the `orderByProperty` query param must
/// be present in the request with the exact IRI value (URL-encoded by reqwest).
/// The wiremock mock is gated on the exact query-param value — if the param is
/// absent or has a different value, the mock does not match and the test fails.
#[tokio::test]
async fn order_by_property_present_when_order_by_is_some() {
    let server = MockServer::start().await;

    // A real property IRI from the incunabula data-model.
    const PROP_IRI: &str = "http://api.dasch.swiss/ontology/0803/incunabula/v2#pagenum";

    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .and(query_param("orderByProperty", PROP_IRI))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let prop_iri = PROP_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(
            &uri,
            PROJECT_IRI,
            RESOURCE_CLASS_IRI,
            Some(&prop_iri),
            0,
            None,
        )
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "request must succeed when orderByProperty param is sent, got: {:?}",
        result
    );

    let received = server
        .received_requests()
        .await
        .expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    let query = received[0].url.query().unwrap_or("");
    assert!(
        query.contains("orderByProperty"),
        "query string must contain 'orderByProperty'; got: {query:?}"
    );
}

/// When `order_by = None` is passed, the `orderByProperty` query param must be
/// ABSENT from the request. A wiremock mock gated on `orderByProperty` existence
/// is set to `expect(0)` — it must not be matched when the param is absent.
#[tokio::test]
async fn order_by_property_absent_when_order_by_is_none() {
    let server = MockServer::start().await;

    // A mock that only matches when orderByProperty is present — must get 0 hits.
    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .and(wiremock::matchers::query_param_is_missing(
            "orderByProperty",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "request with order_by=None must succeed, got: {:?}",
        result
    );

    let received = server
        .received_requests()
        .await
        .expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    let query = received[0].url.query().unwrap_or("");
    assert!(
        !query.contains("orderByProperty"),
        "orderByProperty must be ABSENT when order_by=None; got: {query:?}"
    );
}

// ---------------------------------------------------------------------------
// `schema=complex` query param is always sent
// ---------------------------------------------------------------------------

/// The `schema=complex` query parameter must always be present per D4 (switched
/// 2026-06-17 after live verification that complex carries creation/last-modified
/// dates): baked into the HTTP URL, not a trait parameter.
#[tokio::test]
async fn schema_complex_query_param_is_always_sent() {
    let server = MockServer::start().await;

    // Mock requires schema=complex.
    Mock::given(method("GET"))
        .and(path(RESOURCES_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_resources(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, None, 0, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "request must succeed when schema=complex param is sent, got: {:?}",
        result
    );

    let received = server
        .received_requests()
        .await
        .expect("request recording should be enabled");
    assert_eq!(received.len(), 1);
    let query = received[0].url.query().unwrap_or("");
    assert!(
        query.contains("schema=complex"),
        "query must contain 'schema=complex'; got: {query:?}"
    );
}
