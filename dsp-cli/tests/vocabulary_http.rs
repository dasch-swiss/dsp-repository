// HTTP integration tests for `DspClient::list_vocabularies` /
// `DspClient::describe_vocabulary` (plan 034) against `HttpDspClient`.
//
// `reqwest::blocking` is safe alongside `#[tokio::test]` (which wiremock
// requires) provided the blocking client is constructed, used, and dropped
// entirely on a plain OS thread — not on the tokio runtime's thread pool.
// We achieve this with `std::thread::spawn` + `JoinHandle::join`: the
// blocking reqwest runtime lives and dies on its own OS thread, so it never
// tries to drop a Tokio runtime from within an async context (which would
// panic). Do not "fix" this by moving the `HttpDspClient::new()` call back
// into the async body without also dropping the blocking runtime on a
// non-async thread. Mirrors `tests/data_model_list_http.rs`.
//
// Both endpoints are public: an unauthenticated call (token = None) omits
// the Authorization header entirely; an authenticated call sends
// `Authorization: Bearer <token>`.
//
// Fixtures 1–8 are hand-built (ADR-0009's "hand-written for canonical happy
// paths" half of the hybrid-fixture rule). The two `describe_vocabulary_
// recorded_*` tests at the bottom use real, hand-verified recordings from
// `https://api.dasch.swiss` (geoarch project, Period vocabulary) — the
// "recorded for regression cases" half.

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use serde_json::json;
use wiremock::matchers::{header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// A clearly synthetic token — never a real JWT.
const TOKEN: &str = "test-token";

// The project IRI used across the `list_vocabularies` tests (a made-up but
// realistically-shaped project IRI — this test never hits a real server).
const PROJECT_IRI: &str = "http://rdfh.ch/projects/n0eRr0vWTDOArdaBAZ-jQQ";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Percent-encode an IRI the same way `src/client/http.rs`'s `enc()` does:
/// every non-alphanumeric byte becomes `%XX` (uppercase hex).
fn enc(iri: &str) -> String {
    iri.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// Build the expected `/admin/lists/{enc(iri)}` path for `describe_vocabulary`.
fn describe_path(iri: &str) -> String {
    format!("/admin/lists/{}", enc(iri))
}

/// A minimal root-shaped `/admin/lists/{iri}` response body.
fn root_body(root_iri: &str, project_iri: &str, name: &str, children: Vec<serde_json::Value>) -> serde_json::Value {
    json!({
        "type": "ListGetResponseADM",
        "list": {
            "listinfo": {
                "id": root_iri,
                "projectIri": project_iri,
                "name": name,
                "labels": [{"value": name, "language": "en"}],
                "comments": [],
                "isRootNode": true
            },
            "children": children
        }
    })
}

/// A minimal node-shaped `/admin/lists/{iri}` response body. `own_children`
/// is deliberately parseable-but-discarded data: `ListNodeGetDto` on the
/// impl side only models `nodeinfo`, so this payload can never leak into a
/// `VocabularyTree` even if present.
fn node_body(node_iri: &str, root_iri: &str, own_children: Vec<serde_json::Value>) -> serde_json::Value {
    json!({
        "type": "ListNodeGetResponseADM",
        "node": {
            "nodeinfo": {
                "id": node_iri,
                "name": "addressed-node-name",
                "labels": [{"value": "Addressed Node", "language": "en"}],
                "comments": [],
                "position": 2,
                "hasRootNode": root_iri
            },
            "children": own_children
        }
    })
}

/// One child entry for a root/node's `children` array.
fn child_json(
    iri: &str,
    name: &str,
    position: i32,
    root_iri: &str,
    children: Vec<serde_json::Value>,
) -> serde_json::Value {
    json!({
        "id": iri,
        "name": name,
        "labels": [{"value": name, "language": "en"}],
        "comments": [],
        "position": position,
        "hasRootNode": root_iri,
        "children": children
    })
}

// ---------------------------------------------------------------------------
// `list_vocabularies` — happy path
// ---------------------------------------------------------------------------

/// Three varied entries: one with a `name`, multi-language labels and
/// comments; one with `name: null`; one with an untagged label. Asserts
/// `node_count`/`depth` are BOTH `None` — the client never derives them.
#[tokio::test]
async fn list_vocabularies_happy_path_parses_varied_entries() {
    let server = MockServer::start().await;

    let body = json!({
        "lists": [
            {
                "id": "http://rdfh.ch/lists/0838/JbNT7lvfS9yaB5bgbkoa2w",
                "projectIri": PROJECT_IRI,
                "name": "epoch",
                "labels": [
                    {"value": "Period", "language": "en"},
                    {"value": "A3 Period", "language": "de"}
                ],
                "comments": [
                    {"value": "Dating (period)", "language": "en"},
                    {"value": "Datierung (Epoche)", "language": "de"}
                ],
                "isRootNode": true
            },
            {
                "id": "http://rdfh.ch/lists/0838/anon-root",
                "projectIri": PROJECT_IRI,
                "name": null,
                "labels": [],
                "comments": [],
                "isRootNode": true
            },
            {
                "id": "http://rdfh.ch/lists/0838/untagged-root",
                "projectIri": PROJECT_IRI,
                "name": "misc",
                "labels": [
                    {"value": "Miscellaneous"}
                ],
                "comments": [],
                "isRootNode": true
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/admin/lists"))
        .and(query_param("projectIri", PROJECT_IRI))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_vocabularies(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    let vocabs = result.expect("expected Ok");
    assert_eq!(vocabs.len(), 3, "expected 3 vocabularies");

    let epoch = vocabs
        .iter()
        .find(|v| v.header.name.as_deref() == Some("epoch"))
        .expect("epoch must be present");
    assert_eq!(epoch.header.iri, "http://rdfh.ch/lists/0838/JbNT7lvfS9yaB5bgbkoa2w");
    assert_eq!(epoch.header.labels.len(), 2);
    assert!(
        epoch
            .header
            .labels
            .iter()
            .any(|l| l.value == "Period" && l.language.as_deref() == Some("en"))
    );
    assert!(
        epoch
            .header
            .labels
            .iter()
            .any(|l| l.value == "A3 Period" && l.language.as_deref() == Some("de"))
    );
    assert_eq!(epoch.header.comments.len(), 2);
    assert_eq!(epoch.node_count, None, "client never derives node_count");
    assert_eq!(epoch.depth, None, "client never derives depth");

    let anon = vocabs
        .iter()
        .find(|v| v.header.iri == "http://rdfh.ch/lists/0838/anon-root")
        .expect("anon (name: null) root must be present");
    assert_eq!(anon.header.name, None);
    assert!(anon.header.labels.is_empty());
    assert_eq!(anon.node_count, None);
    assert_eq!(anon.depth, None);

    let untagged = vocabs
        .iter()
        .find(|v| v.header.iri == "http://rdfh.ch/lists/0838/untagged-root")
        .expect("untagged-label root must be present");
    assert_eq!(untagged.header.labels.len(), 1);
    assert_eq!(untagged.header.labels[0].value, "Miscellaneous");
    assert_eq!(
        untagged.header.labels[0].language, None,
        "a label with no `language` key must deserialize to an untagged LocalizedText"
    );
    assert_eq!(untagged.node_count, None);
    assert_eq!(untagged.depth, None);
}

// ---------------------------------------------------------------------------
// `list_vocabularies` — empty
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_vocabularies_empty_returns_empty_vec() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/lists"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"lists": []})))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_vocabularies(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    let vocabs = result.expect("empty lists must yield Ok");
    assert!(vocabs.is_empty(), "empty lists array must yield empty Vec");
}

// ---------------------------------------------------------------------------
// `describe_vocabulary` — root-shaped response
// ---------------------------------------------------------------------------

/// A small hand-written root fixture: 2 top-level children, one of which has
/// a nested child — exercises the recursive `ListNodeDto` walk.
#[tokio::test]
async fn describe_vocabulary_root_shaped_response_builds_tree_directly() {
    let server = MockServer::start().await;

    let root_iri = "http://rdfh.ch/lists/0900/hand-built-root";
    let child1_iri = "http://rdfh.ch/lists/0900/child1";
    let child2_iri = "http://rdfh.ch/lists/0900/child2";
    let grandchild_iri = "http://rdfh.ch/lists/0900/grandchild";

    let body = root_body(
        root_iri,
        PROJECT_IRI,
        "handbuilt",
        vec![
            child_json(child1_iri, "child-one", 0, root_iri, vec![]),
            child_json(
                child2_iri,
                "child-two",
                1,
                root_iri,
                vec![child_json(grandchild_iri, "grandchild", 0, root_iri, vec![])],
            ),
        ],
    );

    Mock::given(method("GET"))
        .and(path(describe_path(root_iri)))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_vocabulary(&uri, root_iri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    let tree = result.expect("root-shaped response must parse successfully");
    assert_eq!(tree.root.iri, root_iri);
    assert_eq!(tree.root.name.as_deref(), Some("handbuilt"));
    assert_eq!(tree.project_iri, PROJECT_IRI);
    assert_eq!(tree.requested_node, None, "a root IRI must not set requested_node");
    assert_eq!(tree.children.len(), 2);

    let child1 = &tree.children[0];
    assert_eq!(child1.header.iri, child1_iri);
    assert_eq!(child1.position, 0);
    assert!(child1.children.is_empty());

    let child2 = &tree.children[1];
    assert_eq!(child2.header.iri, child2_iri);
    assert_eq!(child2.position, 1);
    assert_eq!(child2.children.len(), 1, "nested child must be recursively parsed");
    assert_eq!(child2.children[0].header.iri, grandchild_iri);
}

// ---------------------------------------------------------------------------
// `describe_vocabulary` — node-shaped response resolves upward (D2)
// ---------------------------------------------------------------------------

/// The addressed IRI is a node. The FIRST fetch returns a node-shaped body
/// whose own `children` (a decoy) must be discarded; the client then issues
/// exactly ONE more GET, to the resolved root IRI (`hasRootNode`) — never to
/// an `/info` route — and builds the tree from THAT response.
#[tokio::test]
async fn describe_vocabulary_node_shaped_response_resolves_upward_to_root() {
    let server = MockServer::start().await;

    let node_iri = "http://rdfh.ch/lists/0900/addressed-node";
    let resolved_root_iri = "http://rdfh.ch/lists/0900/resolved-root";
    let resolved_child_iri = "http://rdfh.ch/lists/0900/resolved-child";
    let decoy_child_iri = "http://rdfh.ch/lists/0900/decoy-child-from-node-response";

    Mock::given(method("GET"))
        .and(path(describe_path(node_iri)))
        .respond_with(ResponseTemplate::new(200).set_body_json(node_body(
            node_iri,
            resolved_root_iri,
            vec![child_json(decoy_child_iri, "decoy", 0, resolved_root_iri, vec![])],
        )))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(describe_path(resolved_root_iri)))
        .respond_with(ResponseTemplate::new(200).set_body_json(root_body(
            resolved_root_iri,
            PROJECT_IRI,
            "resolved",
            vec![child_json(
                resolved_child_iri,
                "resolved-child",
                0,
                resolved_root_iri,
                vec![],
            )],
        )))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_vocabulary(&uri, node_iri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    let tree = result.expect("node-shaped response must resolve upward successfully");

    // The tree is built from the SECOND (root) response — the decoy child
    // from the first (node) response must never appear.
    assert_eq!(tree.root.iri, resolved_root_iri);
    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].header.iri, resolved_child_iri);
    assert!(
        tree.children.iter().all(|c| c.header.iri != decoy_child_iri),
        "the node response's own (discarded) children must not leak into the tree"
    );
    assert_eq!(
        tree.requested_node,
        Some(node_iri.to_string()),
        "requested_node must record the originally-addressed node IRI"
    );

    // Exactly 2 requests, in order: the addressed node, then its resolved
    // root — and NEVER `/admin/lists/{root}/info`.
    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 2, "exactly two requests must have been made");
    assert_eq!(received[0].url.path(), describe_path(node_iri));
    assert_eq!(received[1].url.path(), describe_path(resolved_root_iri));
    assert!(
        !received.iter().any(|r| r.url.path().ends_with("/info")),
        "the /admin/lists/{{iri}}/info route must NEVER be called"
    );
}

// ---------------------------------------------------------------------------
// `describe_vocabulary` — node resolving to ANOTHER node (hard error)
// ---------------------------------------------------------------------------

/// One resolution hop only, no retry loop: if the `hasRootNode` fetch itself
/// returns a node-shaped body (not a root), the call must fail rather than
/// resolving again.
#[tokio::test]
async fn describe_vocabulary_node_resolving_to_another_node_is_server_error() {
    let server = MockServer::start().await;

    let node_iri = "http://rdfh.ch/lists/0900/addressed-node-2";
    let supposed_root_iri = "http://rdfh.ch/lists/0900/supposed-root-but-is-node";
    let another_root_hint = "http://rdfh.ch/lists/0900/yet-another-root";

    Mock::given(method("GET"))
        .and(path(describe_path(node_iri)))
        .respond_with(ResponseTemplate::new(200).set_body_json(node_body(node_iri, supposed_root_iri, vec![])))
        .mount(&server)
        .await;

    // The "resolved root" is ITSELF another node response.
    Mock::given(method("GET"))
        .and(path(describe_path(supposed_root_iri)))
        .respond_with(ResponseTemplate::new(200).set_body_json(node_body(supposed_root_iri, another_root_hint, vec![])))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_vocabulary(&uri, node_iri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    match result {
        Err(Diagnostic::ServerError(msg)) => {
            assert!(
                msg.to_lowercase().contains("node") || msg.to_lowercase().contains("root"),
                "error message should mention the failed node-to-root resolution, got: {msg}"
            );
        }
        other => panic!("expected Err(Diagnostic::ServerError), got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Non-2xx status → ServerError
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_vocabularies_server_error_500_returns_server_error_diagnostic() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/lists"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_vocabularies(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 500 response");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "500 must map to Diagnostic::ServerError"
    );
}

/// The harder-to-reach path: a 500 on the FIRST fetch of `describe_vocabulary`
/// (before any node-to-root resolution can even be attempted).
#[tokio::test]
async fn describe_vocabulary_first_fetch_server_error_500_returns_server_error_diagnostic() {
    let server = MockServer::start().await;

    let some_iri = "http://rdfh.ch/lists/0900/whatever";

    Mock::given(method("GET"))
        .and(path(describe_path(some_iri)))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_vocabulary(&uri, some_iri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 500 on the first fetch");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "500 must map to Diagnostic::ServerError"
    );
}

// ---------------------------------------------------------------------------
// Malformed body → ServerError (not a panic)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn describe_vocabulary_malformed_body_invalid_json_returns_server_error() {
    let server = MockServer::start().await;

    let some_iri = "http://rdfh.ch/lists/0900/malformed";

    Mock::given(method("GET"))
        .and(path(describe_path(some_iri)))
        .respond_with(ResponseTemplate::new(200).set_body_string("this is not json at all"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_vocabulary(&uri, some_iri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for malformed (non-JSON) body");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "malformed body must map to Diagnostic::ServerError, not panic"
    );
}

/// Valid JSON, but with neither `"list"` nor `"node"` at the top level —
/// exercises the untagged-enum failure path (`ListGetResponseDto` matches
/// neither variant).
#[tokio::test]
async fn describe_vocabulary_body_missing_list_and_node_keys_returns_server_error() {
    let server = MockServer::start().await;

    let some_iri = "http://rdfh.ch/lists/0900/neither-shape";

    Mock::given(method("GET"))
        .and(path(describe_path(some_iri)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"type": "SomethingElse", "unrelated": true})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_vocabulary(&uri, some_iri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err when the body has neither 'list' nor 'node'");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "an unrecognised body shape must map to Diagnostic::ServerError, not panic"
    );
}

// ---------------------------------------------------------------------------
// Bearer-token propagation — `list_vocabularies` (full trio)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_vocabularies_bearer_present_when_token_is_some() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/lists"))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"lists": []})))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_vocabularies(&uri, PROJECT_IRI, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok with token, got: {result:?}");

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    let auth = received[0]
        .headers
        .get("authorization")
        .expect("Authorization header must be present when token is Some");
    assert_eq!(
        auth.to_str().expect("header should be valid UTF-8"),
        format!("Bearer {TOKEN}"),
        "bearer token must match the value passed to list_vocabularies"
    );
}

#[tokio::test]
async fn list_vocabularies_bearer_absent_when_token_is_none() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/lists"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"lists": []})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_vocabularies(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    assert!(
        received[0].headers.get("authorization").is_none(),
        "Authorization header must be ABSENT when token is None"
    );
}

/// Complementary bearer-absent check: a mock gated on the Authorization
/// header must receive zero hits when `token = None`.
#[tokio::test]
async fn list_vocabularies_no_token_does_not_match_bearer_gated_mock() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/admin/lists"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"lists": []})))
        .expect(0) // Must NOT be called when token is None.
        .mount(&server)
        .await;

    // Fallback (no auth requirement).
    Mock::given(method("GET"))
        .and(path("/admin/lists"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"lists": []})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.list_vocabularies(&uri, PROJECT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(
        result.is_ok(),
        "list_vocabularies(None) must succeed via fallback mock, got: {result:?}"
    );
    // The `expect(0)` on the bearer-gated mock is verified at drop.
}

// ---------------------------------------------------------------------------
// Bearer-token propagation — `describe_vocabulary` (lighter present/absent pair)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn describe_vocabulary_bearer_present_when_token_is_some() {
    let server = MockServer::start().await;

    let iri = "http://rdfh.ch/lists/0900/bearer-root";

    Mock::given(method("GET"))
        .and(path(describe_path(iri)))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(root_body(iri, PROJECT_IRI, "bearer-test", vec![])))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_vocabulary(&uri, iri, Some(TOKEN))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok with token, got: {result:?}");
}

#[tokio::test]
async fn describe_vocabulary_bearer_absent_when_token_is_none() {
    let server = MockServer::start().await;

    let iri = "http://rdfh.ch/lists/0900/bearer-root-none";

    Mock::given(method("GET"))
        .and(path(describe_path(iri)))
        .respond_with(ResponseTemplate::new(200).set_body_json(root_body(iri, PROJECT_IRI, "bearer-test", vec![])))
        .mount(&server)
        .await;

    let uri = server.uri();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_vocabulary(&uri, iri, None)
    })
    .join()
    .expect("blocking thread should not panic");

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1, "exactly one request must have been made");
    assert!(
        received[0].headers.get("authorization").is_none(),
        "Authorization header must be ABSENT when token is None"
    );
}

// ---------------------------------------------------------------------------
// Recorded-fixture regression tests (ADR-0009 hybrid-fixture rule)
// ---------------------------------------------------------------------------
//
// Recorded 2026-07-30, unauthenticated, body-only, from `https://api.dasch.swiss`
// — geoarch project's `epoch` / Period vocabulary. Root IRI
// `http://rdfh.ch/lists/0838/JbNT7lvfS9yaB5bgbkoa2w`.

const FIXTURE_ROOT_IRI: &str = "http://rdfh.ch/lists/0838/JbNT7lvfS9yaB5bgbkoa2w";
const FIXTURE_NODE_IRI: &str = "http://rdfh.ch/lists/0838/Wf0OE7L2Tgu7OwCrMSgIwA";
const FIXTURE_PROJECT_IRI: &str = "http://rdfh.ch/projects/n0eRr0vWTDOArdaBAZ-jQQ";

/// The recorded ROOT fixture. `(33, 2)` is VERIFIED ground truth for this
/// exact fixture — computed directly via `VocabularyTree::count_and_depth`
/// against this data, not the plan's informal "3 levels" prose elsewhere.
#[tokio::test]
async fn describe_vocabulary_recorded_root_fixture_regression() {
    let server = MockServer::start().await;

    let body: serde_json::Value = serde_json::from_str(include_str!("fixtures/vocabulary_period_root.json"))
        .expect("recorded root fixture must be valid JSON");

    Mock::given(method("GET"))
        .and(path(describe_path(FIXTURE_ROOT_IRI)))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_vocabulary(&uri, FIXTURE_ROOT_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    let tree = result.expect("recorded root fixture must parse successfully");
    assert_eq!(
        tree.count_and_depth(None),
        (33, 2),
        "recorded geoarch Period fixture must yield 33 nodes across 2 levels"
    );
    assert_eq!(tree.project_iri, FIXTURE_PROJECT_IRI);
    assert_eq!(tree.root.name.as_deref(), Some("epoch"));
    assert_eq!(tree.requested_node, None, "a root IRI must not set requested_node");
}

/// The recorded NODE fixture resolves upward to the SAME recorded root
/// fixture. Asserts `requested_node` is set to the originally-addressed
/// node IRI and that the resulting tree came from the root fixture (same
/// `count_and_depth` as the direct-root test above).
#[tokio::test]
async fn describe_vocabulary_recorded_node_fixture_resolves_to_recorded_root() {
    let server = MockServer::start().await;

    let node_json: serde_json::Value = serde_json::from_str(include_str!("fixtures/vocabulary_period_node.json"))
        .expect("recorded node fixture must be valid JSON");
    let root_json: serde_json::Value = serde_json::from_str(include_str!("fixtures/vocabulary_period_root.json"))
        .expect("recorded root fixture must be valid JSON");

    Mock::given(method("GET"))
        .and(path(describe_path(FIXTURE_NODE_IRI)))
        .respond_with(ResponseTemplate::new(200).set_body_json(node_json))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(describe_path(FIXTURE_ROOT_IRI)))
        .respond_with(ResponseTemplate::new(200).set_body_json(root_json))
        .expect(1)
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.describe_vocabulary(&uri, FIXTURE_NODE_IRI, None)
    })
    .join()
    .expect("blocking thread should not panic");

    let tree = result.expect("recorded node fixture resolving to recorded root must succeed");
    assert_eq!(
        tree.requested_node,
        Some(FIXTURE_NODE_IRI.to_string()),
        "requested_node must record the originally-addressed node IRI"
    );
    assert_eq!(
        tree.count_and_depth(None),
        (33, 2),
        "the resolved tree must come from the ROOT fixture — same node count as the direct-root test"
    );
}
