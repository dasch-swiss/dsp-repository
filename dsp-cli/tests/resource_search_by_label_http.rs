// Wire tests for `DspClient::search_resources_by_label` (`resource list --filter`).
//
// The blocking reqwest client runs on its own OS thread (`std::thread::spawn` +
// `join`), never on the tokio runtime wiremock needs; see `resource_list_http.rs`
// for why.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::ResourcePage;
use serde_json::json;
use wiremock::matchers::{header, method, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "test-token";
const PROJECT_IRI: &str = "http://rdfh.ch/projects/0110";
const RESOURCE_CLASS_IRI: &str = "http://api.dasch.swiss/ontology/0110/h-steiner/v2#ContactSheet";
const SEARCH_PATH: &str = "^/v2/searchbylabel/[^/]+$";

/// Run `search_resources_by_label` against `uri` on a plain OS thread.
fn search(uri: String, term: &'static str, page: u32, token: Option<&'static str>) -> Result<ResourcePage, Diagnostic> {
    std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.search_resources_by_label(&uri, PROJECT_IRI, RESOURCE_CLASS_IRI, term, page, token)
    })
    .join()
    .expect("blocking thread should not panic")
}

/// The request scopes the search to the project and the resource-type, sends
/// the page as `offset`, and the response parses like a resource listing.
#[tokio::test]
async fn request_carries_scope_and_page_and_response_parses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(SEARCH_PATH))
        .and(query_param("limitToProject", PROJECT_IRI))
        .and(query_param("limitToResourceClass", RESOURCE_CLASS_IRI))
        .and(query_param("offset", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "@graph": [
                {
                    "@id": "http://rdfh.ch/0110/aaa",
                    "@type": "h-steiner:ContactSheet",
                    "rdfs:label": "0101a_001",
                    "knora-api:creationDate": { "@value": "2023-01-01T00:00:00Z" },
                },
                {
                    "@id": "http://rdfh.ch/0110/bbb",
                    "@type": "h-steiner:ContactSheet",
                    "rdfs:label": "0101a_002",
                },
            ],
            "knora-api:mayHaveMoreResults": true,
        })))
        .expect(1)
        .mount(&server)
        .await;

    let page = search(server.uri(), "0101a", 2, None).expect("expected Ok");

    assert!(page.may_have_more_results);
    let labels: Vec<&str> = page.resources.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, vec!["0101a_001", "0101a_002"]);
    assert_eq!(page.resources[0].iri, "http://rdfh.ch/0110/aaa");
    assert_eq!(page.resources[0].resource_type, "ContactSheet");
    assert_eq!(page.resources[0].creation_date.as_deref(), Some("2023-01-01T00:00:00Z"));
}

/// The term is one percent-encoded path segment: a `/`, `?` or `#` in it cannot
/// change the URL's structure, and the space between words survives.
#[tokio::test]
async fn term_is_encoded_as_one_path_segment() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(SEARCH_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let page = search(server.uri(), "a/b?c#d e", 0, None).expect("expected Ok");
    assert!(page.resources.is_empty());

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received[0].url.path(), "/v2/searchbylabel/a%2Fb%3Fc%23d%20e");
    assert_eq!(received[0].url.fragment(), None);
}

/// A token is sent as a bearer; no token sends no `Authorization` header.
#[tokio::test]
async fn bearer_only_when_token_is_some() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(SEARCH_PATH))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(SEARCH_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    search(server.uri(), "folio", 0, Some(TOKEN)).expect("expected Ok with token");
    search(server.uri(), "folio", 0, None).expect("expected Ok without token");

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 2);
    assert!(received[1].headers.get("authorization").is_none());
}

/// A 400 is the server refusing the term: a usage error that carries the
/// server's reason, sanitised.
#[tokio::test]
async fn bad_request_is_usage_error_with_server_reason() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(SEARCH_PATH))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "knora-api:error": "dsp.errors.BadRequestException: A search value is expected to have at least length of 3\u{1b}[31m",
        })))
        .mount(&server)
        .await;

    match search(server.uri(), "abc", 0, None) {
        Err(Diagnostic::Usage(msg)) => {
            assert!(msg.contains("at least length of 3"), "server reason missing: {msg}");
            assert!(!msg.contains('\u{1b}'), "control characters must be stripped: {msg:?}");
        }
        other => panic!("expected Usage, got {other:?}"),
    }
}

/// A 5xx maps to `ServerError`, as for the listing.
#[tokio::test]
async fn server_error_is_server_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(SEARCH_PATH))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    assert!(matches!(
        search(server.uri(), "folio", 0, None),
        Err(Diagnostic::ServerError(_))
    ));
}
