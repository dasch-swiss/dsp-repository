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
// All fixture IRIs use real DaSCH project shapes (incunabula project 0803).
// The endpoint is `GET /v2/resources/<percent-encoded-iri>?schema=complex`.
//
// ADR-0001 vocabulary guard: no DSP-API permission codes or group names
// (`knora-admin:`, raw `RV`/`V`/`CR`) in any parsed domain-model assertion.
// All translation happens at the client boundary.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::{ResourceAccess, ResourceVisibility};
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// A clearly synthetic token — never a real JWT.
const TOKEN: &str = "test-token";

// The resource IRI used across the happy-path tests (real incunabula:Page on dev).
const RESOURCE_IRI: &str = "http://rdfh.ch/0803/--6Esp4SVnGG1DBzFvYErw";

// The percent-encoded form of RESOURCE_IRI under NON_ALPHANUMERIC.
// Computed by the same `enc()` helper the HTTP client uses:
// `utf8_percent_encode(iri, NON_ALPHANUMERIC)`.
// Every non-alphanumeric character (`/`, `:`, `.`, `-`) is encoded.
const RESOURCE_IRI_ENCODED: &str = "http%3A%2F%2Frdfh%2Ech%2F0803%2F%2D%2D6Esp4SVnGG1DBzFvYErw";

// Full path the server should see.
const RESOURCE_PATH: &str = "/v2/resources/http%3A%2F%2Frdfh%2Ech%2F0803%2F%2D%2D6Esp4SVnGG1DBzFvYErw";

// ---------------------------------------------------------------------------
// Happy-path fixture — the verified complex-schema single-resource body
// from the real incunabula:Page on dev (trimmed of value fields).
// See plan Step 10 and D4.
// ---------------------------------------------------------------------------

fn happy_body() -> serde_json::Value {
    json!({
        "@id": "http://rdfh.ch/0803/--6Esp4SVnGG1DBzFvYErw",
        "@type": "incunabula:Page",
        "rdfs:label": "n6r",
        "knora-api:arkUrl": {
            "@value": "https://ark.stage.dasch.swiss/ark:/72163/1/0803/==6Esp4SVnGG1DBzFvYErwr",
            "@type": "xsd:anyURI"
        },
        "knora-api:creationDate": {
            "@value": "2011-04-14T07:32:49Z",
            "@type": "xsd:dateTimeStamp"
        },
        "knora-api:attachedToProject": {
            "@id": "http://rdfh.ch/projects/3ABR_2i8QYGSIDvmP9mlEw"
        },
        "knora-api:attachedToUser": {
            "@id": "http://rdfh.ch/users/IShOmBIGSnO1TXd4-Ty4Sw"
        },
        "knora-api:hasPermissions": "CR knora-admin:Creator,knora-admin:ProjectAdmin|V knora-admin:KnownUser,knora-admin:UnknownUser",
        "knora-api:userHasPermission": "V",
        "@context": {
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
            "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
            "incunabula": "http://0.0.0.0:3333/ontology/0803/incunabula/v2#"
        }
    })
}

// ---------------------------------------------------------------------------
// (a) Request shape: path uses the pre-encoded IRI and schema=complex is sent
// ---------------------------------------------------------------------------

/// The GET path sent to the server must be
/// `/v2/resources/<percent-encoded-iri>` with `schema=complex`.
///
/// This guards that:
/// - `enc()` is applied to the IRI before constructing the URL.
/// - The `schema=complex` query param is always appended (D4).
#[tokio::test]
async fn request_path_uses_percent_encoded_iri() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(query_param("schema", "complex"))
        .respond_with(ResponseTemplate::new(200).set_body_json(happy_body()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "happy-path describe_resource must succeed; got: {:?}", result);

    // Recorded request check: path and schema param.
    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1);
    let url = &received[0].url;
    assert!(
        url.path().contains(RESOURCE_IRI_ENCODED),
        "request path must contain the percent-encoded IRI; got path: {:?}",
        url.path()
    );
    let query = url.query().unwrap_or("");
    assert!(
        query.contains("schema=complex"),
        "request query must contain 'schema=complex'; got: {query:?}"
    );
}

// ---------------------------------------------------------------------------
// (b) Happy-path envelope parse: all fields populated correctly
// ---------------------------------------------------------------------------

/// Full happy-path assertions on the parsed `ResourceDetail`.
///
/// Guards label, iri, resource_type (derived from `@type` bare string),
/// ark_url, creation_date (last_modified is legitimately absent in this
/// fixture), attached_project, owner, and the derived visibility/access.
#[tokio::test]
async fn happy_path_parses_full_envelope() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(happy_body()))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic");

    let detail = result.expect("happy path must return Ok(ResourceDetail)");

    assert_eq!(detail.label, "n6r", "label must match rdfs:label");
    assert_eq!(detail.iri, RESOURCE_IRI, "iri must match the @id from the response");
    // resource_type: derived via local_name() from @type bare string "incunabula:Page" → "Page".
    assert_eq!(
        detail.resource_type, "Page",
        "resource_type must be the local name of @type; got: {:?}",
        detail.resource_type
    );
    assert_eq!(
        detail.ark_url.as_deref(),
        Some("https://ark.stage.dasch.swiss/ark:/72163/1/0803/==6Esp4SVnGG1DBzFvYErwr"),
        "ark_url must be extracted from knora-api:arkUrl @value"
    );
    assert_eq!(
        detail.creation_date.as_deref(),
        Some("2011-04-14T07:32:49Z"),
        "creation_date must be extracted from knora-api:creationDate @value"
    );
    // last_modified: legitimately absent in this fixture (never modified resource).
    assert!(
        detail.last_modified.is_none(),
        "last_modified must be None when knora-api:lastModificationDate is absent"
    );
    assert_eq!(
        detail.attached_project.as_deref(),
        Some("http://rdfh.ch/projects/3ABR_2i8QYGSIDvmP9mlEw"),
        "attached_project must be extracted from knora-api:attachedToProject @id"
    );
    assert_eq!(
        detail.owner.as_deref(),
        Some("http://rdfh.ch/users/IShOmBIGSnO1TXd4-Ty4Sw"),
        "owner must be extracted from knora-api:attachedToUser @id"
    );
    // Visibility: KnownUser and UnknownUser both have V → public.
    assert_eq!(
        detail.visibility,
        Some(ResourceVisibility::Public),
        "visibility must be Public (UnknownUser has V in the ACL); got: {:?}",
        detail.visibility
    );
    // Your access: userHasPermission = "V" → View.
    assert_eq!(
        detail.your_access,
        Some(ResourceAccess::View),
        "your_access must be View (userHasPermission = V); got: {:?}",
        detail.your_access
    );
}

// ---------------------------------------------------------------------------
// (c) @type as array — same parsing as bare string
// ---------------------------------------------------------------------------

/// Guards that `@type` as an array (the common complex-schema form) is
/// parsed correctly, mirroring `resource_list_http.rs` test
/// `single_result_no_graph_parses_to_one_resource`.
#[tokio::test]
async fn type_as_array_is_parsed_correctly() {
    let server = MockServer::start().await;

    let body = json!({
        "@id": RESOURCE_IRI,
        // @type as array — the form the server usually returns in complex schema.
        "@type": [
            "http://api.dasch.swiss/ontology/0803/incunabula/v2#page",
            "http://api.knora.org/ontology/knora-api/v2#Resource"
        ],
        "rdfs:label": "Folio Array Test",
        "knora-api:hasPermissions": "V knora-admin:UnknownUser",
        "knora-api:userHasPermission": "V",
        "@context": {
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#"
        }
    });

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let detail = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic")
    .expect("array @type must parse without error");

    // resource_type is derived from the local name of @type[0].
    assert_eq!(
        detail.resource_type, "page",
        "resource_type from array @type must be local name of first element; got: {:?}",
        detail.resource_type
    );
    assert_eq!(detail.label, "Folio Array Test");
}

// ---------------------------------------------------------------------------
// (d) 404 → Diagnostic::NotFound
// ---------------------------------------------------------------------------

/// A 404 response must map to `Diagnostic::NotFound`.
#[tokio::test]
async fn status_404_returns_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "404 must return Err");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::NotFound(_)),
        "404 must map to Diagnostic::NotFound"
    );
}

// ---------------------------------------------------------------------------
// (e) 403 → Diagnostic::AuthRequired
// ---------------------------------------------------------------------------

/// A 403 response must map to `Diagnostic::AuthRequired` (not ServerError).
///
/// An anonymous caller describing a private resource gets 403; the right UX
/// is the auth-required exit code (3) with a "log in" hint, not a server error.
/// This deviates from `describe_project` which lets 403 fall through to
/// `map_unexpected_status`; here it is deliberate (auth-optional instance read).
#[tokio::test]
async fn status_403_returns_auth_required() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "403 must return Err");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::AuthRequired(_)),
        "403 must map to Diagnostic::AuthRequired (not ServerError)"
    );
}

// ---------------------------------------------------------------------------
// (f) 401 → Diagnostic::AuthRequired
// ---------------------------------------------------------------------------

/// A 401 response must also map to `Diagnostic::AuthRequired`.
#[tokio::test]
async fn status_401_returns_auth_required() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "401 must return Err");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::AuthRequired(_)),
        "401 must map to Diagnostic::AuthRequired"
    );
}

// ---------------------------------------------------------------------------
// (g) 500 → Diagnostic::ServerError
// ---------------------------------------------------------------------------

/// A 500 response maps to `Diagnostic::ServerError`.
#[tokio::test]
async fn status_500_returns_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "500 must return Err");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "500 must map to Diagnostic::ServerError"
    );
}

// ---------------------------------------------------------------------------
// (h) Bearer present when token is Some
// ---------------------------------------------------------------------------

/// When `token = Some("test-token")` is passed, the request carries an
/// `Authorization: Bearer test-token` header.
#[tokio::test]
async fn bearer_present_when_token_is_some() {
    use wiremock::matchers::header;

    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(happy_body()))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, Some(TOKEN), false)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "describe_resource with token must succeed; got: {:?}", result);

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1);
    let auth = received[0]
        .headers
        .get("authorization")
        .expect("Authorization header must be present when token is Some");
    assert_eq!(
        auth.to_str().expect("header should be valid UTF-8"),
        format!("Bearer {TOKEN}"),
        "bearer token must match the value passed to describe_resource"
    );
}

// ---------------------------------------------------------------------------
// (i) Bearer absent when token is None
// ---------------------------------------------------------------------------

/// When `token = None` is passed, NO `Authorization` header must be sent.
#[tokio::test]
async fn bearer_absent_when_token_is_none() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(happy_body()))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic");

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1);
    assert!(
        received[0].headers.get("authorization").is_none(),
        "Authorization header must be ABSENT when token is None"
    );
}

// ---------------------------------------------------------------------------
// (j) Visibility derivation end-to-end: ProjectMembers
// ---------------------------------------------------------------------------

/// A fixture whose `hasPermissions` grants only project groups (no
/// UnknownUser/KnownUser) → `ProjectMembers` visibility.
#[tokio::test]
async fn visibility_project_members_from_project_only_acl() {
    let server = MockServer::start().await;

    let body = json!({
        "@id": RESOURCE_IRI,
        "@type": "incunabula:Page",
        "rdfs:label": "Private Folio",
        // Only Creator and ProjectAdmin — no world groups.
        "knora-api:hasPermissions": "CR knora-admin:Creator,knora-admin:ProjectAdmin",
        "knora-api:userHasPermission": "CR",
        "@context": {
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
            "incunabula": "http://0.0.0.0:3333/ontology/0803/incunabula/v2#"
        }
    });

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let detail = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic")
    .expect("project-members-only fixture must parse without error");

    assert_eq!(
        detail.visibility,
        Some(ResourceVisibility::ProjectMembers),
        "visibility must be ProjectMembers when only project groups are in the ACL; got: {:?}",
        detail.visibility
    );
    assert_eq!(
        detail.your_access,
        Some(ResourceAccess::Manage),
        "your_access must be Manage for CR code; got: {:?}",
        detail.your_access
    );
}

// ---------------------------------------------------------------------------
// (k) Visibility derivation end-to-end: PublicRestricted
// ---------------------------------------------------------------------------

/// A fixture whose `hasPermissions` grants exactly `RV` to `UnknownUser` →
/// `PublicRestricted` visibility.
#[tokio::test]
async fn visibility_public_restricted_from_rv_unknown_user() {
    let server = MockServer::start().await;

    let body = json!({
        "@id": RESOURCE_IRI,
        "@type": "incunabula:Page",
        "rdfs:label": "Restricted View Folio",
        // UnknownUser has only RV (restricted view) — not full V.
        "knora-api:hasPermissions": "CR knora-admin:Creator|RV knora-admin:UnknownUser",
        "knora-api:userHasPermission": "RV",
        "@context": {
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
            "incunabula": "http://0.0.0.0:3333/ontology/0803/incunabula/v2#"
        }
    });

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let detail = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic")
    .expect("public-restricted fixture must parse without error");

    assert_eq!(
        detail.visibility,
        Some(ResourceVisibility::PublicRestricted),
        "visibility must be PublicRestricted when UnknownUser has exactly RV; got: {:?}",
        detail.visibility
    );
    assert_eq!(
        detail.your_access,
        Some(ResourceAccess::RestrictedView),
        "your_access must be RestrictedView for RV code; got: {:?}",
        detail.your_access
    );
}

// ---------------------------------------------------------------------------
// (l) Malformed body → ServerError
// ---------------------------------------------------------------------------

/// A 200 response with an invalid JSON body must yield `ServerError` (not panic).
#[tokio::test]
async fn malformed_body_returns_server_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_string("this is not json"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "malformed body must return Err");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "malformed body must map to Diagnostic::ServerError"
    );
}

// ---------------------------------------------------------------------------
// (l2) Visibility derivation end-to-end: LoggedInUsers
// ---------------------------------------------------------------------------

/// A fixture whose `hasPermissions` grants ≥ RV to `KnownUser` but has NO
/// `UnknownUser` entry → `LoggedInUsers` visibility.
///
/// ACL: "CR knora-admin:Creator,knora-admin:ProjectAdmin|V knora-admin:KnownUser"
/// UnknownUser is ABSENT; KnownUser has V. Per the permission model
/// (RV<V<M<D<CR; world groups are UnknownUser = anonymous, KnownUser = any
/// logged-in user) this is LoggedInUsers.
#[tokio::test]
async fn visibility_logged_in_users_from_known_user_acl() {
    let server = MockServer::start().await;

    let body = json!({
        "@id": RESOURCE_IRI,
        "@type": "incunabula:Page",
        "rdfs:label": "Logged-In-Only Folio",
        // KnownUser has V; UnknownUser is absent entirely.
        "knora-api:hasPermissions": "CR knora-admin:Creator,knora-admin:ProjectAdmin|V knora-admin:KnownUser",
        "knora-api:userHasPermission": "V",
        "@context": {
            "knora-api": "http://api.knora.org/ontology/knora-api/v2#",
            "incunabula": "http://0.0.0.0:3333/ontology/0803/incunabula/v2#"
        }
    });

    Mock::given(method("GET"))
        .and(path(RESOURCE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let uri = server.uri();
    let iri = RESOURCE_IRI.to_string();
    let detail = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic")
    .expect("logged-in-users fixture must parse without error");

    assert_eq!(
        detail.visibility,
        Some(ResourceVisibility::LoggedInUsers),
        "visibility must be LoggedInUsers when UnknownUser is absent and KnownUser has ≥ RV; got: {:?}",
        detail.visibility
    );
    assert_eq!(
        detail.your_access,
        Some(ResourceAccess::View),
        "your_access must be View for V code; got: {:?}",
        detail.your_access
    );
}

// ---------------------------------------------------------------------------
// (m) Connection refused → Diagnostic::Network
// ---------------------------------------------------------------------------

/// A connection failure (nothing listening) must map to `Diagnostic::Network`.
#[tokio::test]
async fn connection_refused_returns_network() {
    let uri = "http://127.0.0.1:1".to_string();
    let iri = RESOURCE_IRI.to_string();

    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_resource(&uri, &iri, None, false)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "connection refused must return Err");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::Network(_)),
        "connection refused must map to Diagnostic::Network"
    );
}
