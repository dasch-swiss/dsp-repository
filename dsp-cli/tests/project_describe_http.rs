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
// The `GET /admin/projects/{type}/{id}` endpoint is public: an unauthenticated
// call (token = None) omits the Authorization header entirely; an authenticated
// call sends `Authorization: Bearer <token>`. Tests assert both behaviours.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::{DataModelSummary, ProjectStatus};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// A clearly synthetic token — never a real JWT.
const TOKEN: &str = "test-token";

// ---------------------------------------------------------------------------
// Realistic beol fixture (grounded in live API — see plan §Grounding)
// ---------------------------------------------------------------------------

/// Build the beol-shaped project detail fixture.
///
/// Uses a realistic random-suffix IRI (not shortcode-based, per plan §Grounding),
/// HTML in description, 4 ontologies in non-alphabetical wire order (to verify sorting).
fn beol_body() -> serde_json::Value {
    json!({
        "project": {
            "id": "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF",
            "shortname": "beol",
            "shortcode": "0801",
            "longname": "Bernoulli-Euler Online",
            "description": [
                {
                    "value": "<b>BEOL</b> — early modern mathematics.",
                    "language": "en"
                }
            ],
            "keywords": ["Bernoulli", "Euler", "Mathematics"],
            // Wire order is deliberately NOT alphabetical — verify that the
            // client sorts by name ascending.
            "ontologies": [
                "http://api.dasch.swiss/ontology/0801/leibniz/v2",
                "http://api.dasch.swiss/ontology/0801/biblio/v2",
                "http://api.dasch.swiss/ontology/0801/newton/v2",
                "http://api.dasch.swiss/ontology/0801/beol/v2"
            ],
            "status": true,
            "selfjoin": false
        }
    })
}

// ---------------------------------------------------------------------------
// Happy path — translates DTO correctly incl. sorted data-model names
// ---------------------------------------------------------------------------

/// A 200 response with the beol fixture is translated to the correct
/// `ProjectDetail`, including:
/// - realistic random-suffix IRI
/// - longname, shortname, shortcode
/// - `status: true` → `ProjectStatus::Active`
/// - description value + language
/// - keywords
/// - ontologies → `data_models` sorted by name ascending
#[tokio::test]
async fn happy_path_translates_dto_to_project_detail() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/0801"))
        .respond_with(ResponseTemplate::new(200).set_body_json(beol_body()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_project(&uri, "0801", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let detail = result.unwrap();

    // Identity fields
    assert_eq!(detail.iri, "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF");
    assert_eq!(detail.shortcode, "0801");
    assert_eq!(detail.shortname, "beol");
    assert_eq!(detail.longname.as_deref(), Some("Bernoulli-Euler Online"));

    // Status
    assert_eq!(detail.status, ProjectStatus::Active, "status:true must map to Active");

    // Description
    assert_eq!(detail.description.len(), 1, "one description entry");
    assert_eq!(detail.description[0].value, "<b>BEOL</b> — early modern mathematics.");
    assert_eq!(
        detail.description[0].language.as_deref(),
        Some("en"),
        "language tag must be preserved"
    );

    // Keywords
    assert_eq!(detail.keywords, vec!["Bernoulli", "Euler", "Mathematics"]);

    // Data-models: sorted by name ascending, not wire order
    assert_eq!(detail.data_models.len(), 4, "four ontologies → four data_models");
    let names: Vec<&str> = detail.data_models.iter().map(|dm| dm.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["beol", "biblio", "leibniz", "newton"],
        "data-model names must be sorted ascending"
    );

    // Verify that IRIs are preserved alongside names
    let expected: Vec<DataModelSummary> = vec![
        DataModelSummary {
            name: "beol".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
        },
        DataModelSummary {
            name: "biblio".into(),
            iri: "http://api.dasch.swiss/ontology/0801/biblio/v2".into(),
        },
        DataModelSummary {
            name: "leibniz".into(),
            iri: "http://api.dasch.swiss/ontology/0801/leibniz/v2".into(),
        },
        DataModelSummary {
            name: "newton".into(),
            iri: "http://api.dasch.swiss/ontology/0801/newton/v2".into(),
        },
    ];
    assert_eq!(detail.data_models, expected, "full DataModelSummary match");
}

// ---------------------------------------------------------------------------
// 404 → Diagnostic::NotFound with describe-specific hint
// ---------------------------------------------------------------------------

/// A 404 response maps to `Diagnostic::NotFound` with a message that contains
/// the project input and the `dsp vre project list` recovery hint.
#[tokio::test]
async fn not_found_returns_not_found_diagnostic_with_hint() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/9999"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let uri_clone = uri.clone();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_project(&uri, "9999", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 404 response");
    let err = result.unwrap_err();
    assert!(
        matches!(err, Diagnostic::NotFound(_)),
        "404 must map to Diagnostic::NotFound, got: {err:?}"
    );
    if let Diagnostic::NotFound(msg) = err {
        assert!(msg.contains("9999"), "error message must contain the project input '9999'");
        assert!(
            msg.contains("dsp vre project list"),
            "error message must contain the recovery hint 'dsp vre project list'"
        );
        assert!(msg.contains(&uri_clone), "error message must contain the server");
    }
}

// ---------------------------------------------------------------------------
// Malformed success body → Diagnostic::ServerError
// ---------------------------------------------------------------------------

/// A 200 response with a body that cannot be parsed as `ProjectDetailApiResponse`
/// maps to `Diagnostic::ServerError`.
#[tokio::test]
async fn malformed_success_body_returns_server_error() {
    let server = MockServer::start().await;

    // Return a 200 with a body that is valid JSON but missing required fields
    // (e.g., `status` is absent — it has no `#[serde(default)]`).
    let malformed = json!({
        "project": {
            "id": "http://rdfh.ch/projects/0001",
            "shortcode": "0001",
            "shortname": "anything"
            // "status" is deliberately absent — must fail parse loudly
        }
    });

    Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/0001"))
        .respond_with(ResponseTemplate::new(200).set_body_json(malformed))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_project(&uri, "0001", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "missing `status` field must cause a parse error");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "parse failure on 200 must map to Diagnostic::ServerError"
    );
}

// ---------------------------------------------------------------------------
// Bearer-token forwarding — mirrors project_list_http.rs pattern
// ---------------------------------------------------------------------------

/// When `token = Some("test-token")` is passed, the request carries
/// `Authorization: Bearer test-token`.
#[tokio::test]
async fn bearer_present_when_token_is_some() {
    use wiremock::matchers::header;

    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/0801"))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(beol_body()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_project(&uri, "0801", Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok when bearer token is forwarded, got: {:?}", result);
}

/// When `token = None` is passed, the request has NO `Authorization` header.
#[tokio::test]
async fn bearer_absent_when_token_is_none() {
    use wiremock::matchers::header_exists;

    let server = MockServer::start().await;

    // A mock that expects 0 hits when Authorization is present.
    Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/0801"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(beol_body()))
        .expect(0) // Must NOT be called when token is None.
        .mount(&server)
        .await;

    // Fallback mock without auth requirement.
    Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/0801"))
        .respond_with(ResponseTemplate::new(200).set_body_json(beol_body()))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_project(&uri, "0801", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "describe_project(None) must succeed via fallback mock, got: {:?}",
        result
    );
    // The `expect(0)` on the bearer-gated mock is verified at drop.
}

// ---------------------------------------------------------------------------
// Network failure → Diagnostic::Network
// ---------------------------------------------------------------------------

/// A connection failure (nothing listening) must map to `Diagnostic::Network`.
#[tokio::test]
async fn describe_project_connection_refused_returns_network() {
    let uri = "http://127.0.0.1:1".to_string();

    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_project(&uri, "0801", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err when server is unreachable");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::Network(_)),
        "connection failure must map to Diagnostic::Network"
    );
}

// ---------------------------------------------------------------------------
// status: false → ProjectStatus::Inactive
// ---------------------------------------------------------------------------

/// A 200 response with `status: false` maps to `ProjectStatus::Inactive`.
/// The happy-path test covers `status: true → Active`; this covers the other branch.
#[tokio::test]
async fn status_false_maps_to_inactive() {
    let server = MockServer::start().await;

    let body = json!({
        "project": {
            "id": "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF",
            "shortname": "beol",
            "shortcode": "0801",
            "longname": "Bernoulli-Euler Online",
            "description": [],
            "keywords": [],
            "ontologies": [],
            "status": false,
            "selfjoin": false
        }
    });

    Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/0801"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_project(&uri, "0801", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let detail = result.unwrap();
    assert_eq!(detail.status, ProjectStatus::Inactive, "status:false must map to Inactive");
}

// ---------------------------------------------------------------------------
// Over-80-char input that 404s → truncated message with ellipsis
// ---------------------------------------------------------------------------

/// A project input longer than 80 chars that results in a 404 must produce a
/// `Diagnostic::NotFound` whose message contains a `…` (ellipsis) and does NOT
/// contain the full over-length input verbatim.
/// Plan: "Cap the echoed input at 80 chars with a `…` suffix."
#[tokio::test]
async fn long_input_404_message_is_truncated_with_ellipsis() {
    let server = MockServer::start().await;

    // 85-char fake shortname — deliberately longer than 80 chars.
    let long_input = "a".repeat(85);

    Mock::given(method("GET"))
        .and(path(format!("/admin/projects/shortname/{long_input}")))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let long_input_clone = long_input.clone();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_project(&uri, &long_input_clone, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 404 response");
    let err = result.unwrap_err();
    assert!(
        matches!(err, Diagnostic::NotFound(_)),
        "404 must map to Diagnostic::NotFound, got: {err:?}"
    );
    if let Diagnostic::NotFound(msg) = err {
        assert!(
            msg.contains('…'),
            "error message must contain '…' (ellipsis truncation) for over-80-char input; got: {msg:?}"
        );
        assert!(
            !msg.contains(&long_input),
            "error message must NOT contain the full over-length input verbatim; got: {msg:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Shortname and IRI routing — URL path must be correct
// ---------------------------------------------------------------------------

/// describe_project with a shortname input routes to `/admin/projects/shortname/beol`.
#[tokio::test]
async fn shortname_input_routes_to_shortname_path() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/projects/shortname/beol"))
        .respond_with(ResponseTemplate::new(200).set_body_json(beol_body()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_project(&uri, "beol", None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok for shortname input, got: {:?}", result);
}
