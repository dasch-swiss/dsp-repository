//! `Server` against fakes. Its own test binary: it installs the rustls provider the client needs,
//! which a production binary installs at its start.

use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use vre_bridge_client::{
    ClientError, DeleteOutcome, ExportStatus, ExportTrigger, FileError, Server, ServiceIdentity, Timeouts,
};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const PRIVATE_KEY: &str = include_str!("../testdata/ed25519-private.b64");
const IRI: &str = "http://rdfh.ch/projects/0001";
const ENCODED_IRI: &str = "http%3A%2F%2Frdfh%2Ech%2Fprojects%2F0001";

fn timeouts() -> Timeouts {
    Timeouts {
        connect: Duration::from_millis(500),
        read: Duration::from_millis(500),
        request: Duration::from_millis(500),
        download: Duration::from_millis(800),
        file: Duration::from_millis(500),
    }
}

fn server(base: &str) -> Server {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let identity = ServiceIdentity::new("dsp-repository-test", "test-kid-1", PRIVATE_KEY).unwrap();
    Server::new(base, identity, timeouts()).unwrap()
}

fn has_authorization(request: &Request) -> bool {
    request.headers.contains_key("authorization")
}

async fn received(mock: &MockServer) -> Vec<Request> {
    mock.received_requests().await.unwrap()
}

#[tokio::test]
async fn test_project_iri_reads_the_project_id_without_credentials() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/0803"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "project": {"id": "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF", "shortcode": "0803"}
        })))
        .mount(&mock)
        .await;

    let iri = server(&mock.uri()).project_iri("0803").await.unwrap();

    assert_eq!(iri, "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF");
    assert!(!received(&mock).await.iter().any(has_authorization));
}

#[tokio::test]
async fn test_changes_sends_cutoff_with_z_and_the_project_header_without_credentials() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/resources/info"))
        .and(query_param("cutoff", "2026-11-20T01:02:03.456Z"))
        .and(header("X-Knora-Accept-Project", IRI))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "resources": [], "count": 3, "asOf": "2026-11-20T01:03:03.456789Z"
        })))
        .mount(&mock)
        .await;
    let cutoff: DateTime<Utc> = "2026-11-20T02:02:03.456+01:00".parse().unwrap();

    let changes = server(&mock.uri()).changes(IRI, cutoff).await.unwrap();

    assert_eq!(changes.as_of, "2026-11-20T01:03:03.456789Z");
    assert_eq!(changes.count, 3);
    let requests = received(&mock).await;
    assert!(!requests.iter().any(has_authorization));
    assert!(!requests[0].url.query().unwrap_or_default().contains('+'));
}

#[tokio::test]
async fn test_export_sends_skip_assets_the_encoded_iri_as_one_segment_and_a_bearer_token() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v3/projects/{ENCODED_IRI}/exports")))
        .and(query_param("skipAssets", "true"))
        .respond_with(
            ResponseTemplate::new(202).set_body_json(serde_json::json!({"id": "exp-1", "status": "in_progress"})),
        )
        .mount(&mock)
        .await;

    let outcome = server(&mock.uri()).export(IRI).await.unwrap();

    assert_eq!(outcome, ExportTrigger::Started { id: "exp-1".into() });
    let requests = received(&mock).await;
    let authorization = requests[0].headers.get("authorization").unwrap().to_str().unwrap();
    let token = authorization.strip_prefix("Bearer ").unwrap();
    let token_header = jsonwebtoken::decode_header(token).unwrap();
    assert_eq!(token_header.alg, jsonwebtoken::Algorithm::EdDSA);
    assert_eq!(token_header.kid.as_deref(), Some("test-kid-1"));
}

#[tokio::test]
async fn test_export_conflict_export_exists_is_occupied_with_the_details_id() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
            "message": "An export exists",
            "errors": [{"code": "export_exists", "message": "x", "details": {"id": "exp-9", "projectIri": "http://rdfh.ch/projects/other"}}]
        })))
        .mount(&mock)
        .await;

    let outcome = server(&mock.uri()).export(IRI).await.unwrap();

    assert_eq!(outcome, ExportTrigger::Occupied { id: "exp-9".into() });
}

#[tokio::test]
async fn test_export_other_conflict_is_busy() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(409).set_body_string("not json"))
        .mount(&mock)
        .await;

    assert_eq!(server(&mock.uri()).export(IRI).await.unwrap(), ExportTrigger::Busy);
}

#[tokio::test]
async fn test_export_status_maps_404_to_none_and_the_three_status_strings() {
    let mock = MockServer::start().await;
    for (id, status) in [("a", "in_progress"), ("b", "completed"), ("c", "failed")] {
        Mock::given(method("GET"))
            .and(path(format!("/v3/projects/{ENCODED_IRI}/exports/{id}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": id, "status": status})))
            .mount(&mock)
            .await;
    }
    Mock::given(method("GET"))
        .and(path(format!("/v3/projects/{ENCODED_IRI}/exports/gone")))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock)
        .await;
    let server = server(&mock.uri());

    assert_eq!(server.export_status(IRI, "a").await.unwrap(), Some(ExportStatus::InProgress));
    assert_eq!(server.export_status(IRI, "b").await.unwrap(), Some(ExportStatus::Completed));
    assert_eq!(server.export_status(IRI, "c").await.unwrap(), Some(ExportStatus::Failed));
    assert_eq!(server.export_status(IRI, "gone").await.unwrap(), None);
    assert!(received(&mock).await.iter().all(has_authorization));
}

#[tokio::test]
async fn test_a_redirect_is_not_followed() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/admin/projects/shortcode/0803"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", format!("{}/elsewhere", mock.uri())))
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/elsewhere"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"project": {"id": "x"}})))
        .expect(0)
        .mount(&mock)
        .await;

    let error = server(&mock.uri()).project_iri("0803").await.unwrap_err();

    assert!(matches!(error, ClientError::Status { status: 302, .. }), "{error:?}");
}

#[tokio::test]
async fn test_wait_completed_fails_on_failed() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "e", "status": "failed"})))
        .mount(&mock)
        .await;

    let error = server(&mock.uri())
        .wait_completed(IRI, "e", Duration::from_millis(20), Duration::from_millis(500))
        .await
        .unwrap_err();

    assert!(matches!(error, ClientError::ExportFailed), "{error:?}");
}

#[tokio::test]
async fn test_wait_completed_fails_on_the_time_limit() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "e", "status": "in_progress"})))
        .mount(&mock)
        .await;

    let error = server(&mock.uri())
        .wait_completed(IRI, "e", Duration::from_millis(20), Duration::from_millis(200))
        .await
        .unwrap_err();

    assert!(matches!(error, ClientError::TimedOut { .. }), "{error:?}");
    assert!(received(&mock).await.len() > 1);
}

#[tokio::test]
async fn test_wait_finished_returns_once_the_export_is_not_in_progress() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "e", "status": "in_progress"})))
        .up_to_n_times(2)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "e", "status": "failed"})))
        .mount(&mock)
        .await;

    let status = server(&mock.uri())
        .wait_finished(IRI, "e", Duration::from_millis(10), Duration::from_millis(500))
        .await
        .unwrap();

    assert_eq!(status, ExportStatus::Failed);
    assert_eq!(received(&mock).await.len(), 3);
}

#[tokio::test]
async fn test_download_writes_the_body_to_the_file() {
    let mock = MockServer::start().await;
    let body: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    Mock::given(method("GET"))
        .and(path(format!("/v3/projects/{ENCODED_IRI}/exports/e/download")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
        .mount(&mock)
        .await;
    let work = vre_bridge_client::private_work_dir(&std::env::temp_dir()).unwrap();

    let zip = server(&mock.uri()).download(IRI, "e", &work, u64::MAX).await.unwrap();

    assert_eq!(zip.size, body.len() as u64);
    assert_eq!(zip.path, work.path().join("dump.zip"));
    assert_eq!(std::fs::read(&zip.path).unwrap(), body);
    assert!(received(&mock).await.iter().all(has_authorization));
}

#[tokio::test]
async fn test_download_fails_on_a_short_body() {
    // A server that promises 100 bytes, sends 10 and closes the connection.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 4096];
        let _ = socket.read(&mut request).await;
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n0123456789")
            .await
            .unwrap();
        socket.shutdown().await.unwrap();
    });
    let work = vre_bridge_client::private_work_dir(&std::env::temp_dir()).unwrap();

    let result = server(&format!("http://{address}")).download(IRI, "e", &work, u64::MAX).await;

    // hyper itself reports the early end; `ShortBody` covers a body it lets through.
    assert!(
        matches!(result, Err(ClientError::Request { .. } | ClientError::ShortBody { .. })),
        "{result:?}"
    );
}

#[tokio::test]
async fn test_download_of_an_unfinished_export_is_an_error() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(409))
        .mount(&mock)
        .await;
    let work = vre_bridge_client::private_work_dir(&std::env::temp_dir()).unwrap();

    let error = server(&mock.uri()).download(IRI, "e", &work, u64::MAX).await.unwrap_err();

    assert!(matches!(error, ClientError::Status { status: 409, .. }), "{error:?}");
    assert!(!work.path().join("dump.zip").exists());
}

#[tokio::test]
async fn test_download_announced_over_the_cap_fails_before_writing() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; 101]))
        .mount(&mock)
        .await;
    let work = vre_bridge_client::private_work_dir(&std::env::temp_dir()).unwrap();

    let error = server(&mock.uri()).download(IRI, "e", &work, 100).await.unwrap_err();

    assert!(matches!(error, ClientError::TooLarge { limit: 100, .. }), "{error:?}");
    assert!(!work.path().join("dump.zip").exists());
}

#[tokio::test]
async fn test_download_streaming_past_the_cap_without_a_length_fails() {
    // A chunked body announces no length, so only the bytes received tell.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 4096];
        let _ = socket.read(&mut request).await;
        let chunk = "0123456789";
        let body = format!("a\r\n{chunk}\r\na\r\n{chunk}\r\n0\r\n\r\n");
        let response = format!("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{body}");
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
    });
    let work = vre_bridge_client::private_work_dir(&std::env::temp_dir()).unwrap();

    let result = server(&format!("http://{address}")).download(IRI, "e", &work, 15).await;

    assert!(matches!(result, Err(ClientError::TooLarge { limit: 15, .. })), "{result:?}");
}

#[tokio::test]
async fn test_delete_export_maps_204_409_in_progress_and_404() {
    let mock = MockServer::start().await;
    for (id, response) in [
        ("done", ResponseTemplate::new(204)),
        (
            "running",
            ResponseTemplate::new(409).set_body_json(serde_json::json!({
                "message": "running", "errors": [{"code": "export_in_progress", "message": "x", "details": {}}]
            })),
        ),
        ("gone", ResponseTemplate::new(404)),
        ("odd", ResponseTemplate::new(409).set_body_string("not json")),
    ] {
        Mock::given(method("DELETE"))
            .and(path(format!("/v3/projects/{ENCODED_IRI}/exports/{id}")))
            .respond_with(response)
            .mount(&mock)
            .await;
    }
    let server = server(&mock.uri());

    assert_eq!(server.delete_export(IRI, "done").await.unwrap(), DeleteOutcome::Deleted);
    assert_eq!(server.delete_export(IRI, "running").await.unwrap(), DeleteOutcome::StillRunning);
    assert_eq!(server.delete_export(IRI, "gone").await.unwrap(), DeleteOutcome::Gone);
    let odd = server.delete_export(IRI, "odd").await;
    assert!(matches!(odd, Err(ClientError::Status { status: 409, .. })), "{odd:?}");
    assert!(received(&mock).await.iter().all(has_authorization));
}

#[tokio::test]
async fn test_download_file_sends_no_credentials_and_maps_401_and_403_to_refused() {
    let mock = MockServer::start().await;
    for (name, status) in [
        ("open.txt", 200),
        ("login.txt", 401),
        ("closed.txt", 403),
        ("broken.txt", 500),
    ] {
        Mock::given(method("GET"))
            .and(path(format!("/files/{name}")))
            .respond_with(ResponseTemplate::new(status).set_body_string("text"))
            .mount(&mock)
            .await;
    }
    let server = server(&mock.uri());
    let work = tempfile::tempdir().unwrap();
    let url = |name: &str| format!("{}/files/{name}", mock.uri());
    let into = |name: &str| work.path().join(name);

    assert_eq!(server.download_file(&url("open.txt"), &into("open.txt"), 100).await.unwrap(), 4);
    assert_eq!(std::fs::read(into("open.txt")).unwrap(), b"text");
    assert!(matches!(
        server.download_file(&url("login.txt"), &into("login.txt"), 100).await,
        Err(FileError::Refused(401))
    ));
    assert!(matches!(
        server.download_file(&url("closed.txt"), &into("closed.txt"), 100).await,
        Err(FileError::Refused(403))
    ));
    assert!(matches!(
        server.download_file(&url("broken.txt"), &into("broken.txt"), 100).await,
        Err(FileError::Failed(ClientError::Status { status: 500, .. }))
    ));
    assert!(!received(&mock).await.iter().any(has_authorization));
    let left: Vec<_> = std::fs::read_dir(work.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["open.txt"]);
}

#[tokio::test]
async fn test_download_file_refuses_a_url_off_the_configured_bases_before_any_request() {
    let (api, iiif, elsewhere) = (MockServer::start().await, MockServer::start().await, MockServer::start().await);
    for mock in [&api, &iiif, &elsewhere] {
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("text"))
            .mount(mock)
            .await;
    }
    let server = server(&format!("{}/api", api.uri()))
        .with_file_base(&format!("{}/iiif", iiif.uri()))
        .unwrap();
    let work = tempfile::tempdir().unwrap();
    let host_port = elsewhere.uri().trim_start_matches("http://").to_owned();
    let refused = [
        format!("{}/iiif/0001/a.xml/file", elsewhere.uri()),
        format!("{}/other/0001/a.xml/file", iiif.uri()),
        format!("{}/iiifx/0001/a.xml/file", iiif.uri()),
        format!("{}/api", api.uri()),
        format!("{}/iiif/../other/a.xml", iiif.uri()),
        format!("{}/iiif/%2e%2e%2fother/a.xml", iiif.uri()),
        format!("{}/iiif/a%5Cb.xml", iiif.uri()),
        format!("{}/iiif/a.xml?x=1", iiif.uri()),
        format!("{}/iiif/a.xml#x", iiif.uri()),
        format!("http://user:pass@{host_port}/iiif/a.xml"),
        format!("ftp://{host_port}/iiif/a.xml"),
        "not a url".to_owned(),
    ];

    for url in &refused {
        let result = server.download_file(url, &work.path().join("f"), 100).await;
        assert!(
            matches!(result, Err(FileError::Failed(ClientError::FileUrl))),
            "{url}: {result:?}"
        );
    }

    assert!(received(&elsewhere).await.is_empty());
    assert!(received(&iiif).await.is_empty());
    assert!(received(&api).await.is_empty());
    let into = work.path().join("ok");
    let ok = server
        .download_file(&format!("{}/iiif/0001/a.xml/file", iiif.uri()), &into, 100)
        .await;
    assert_eq!(ok.unwrap(), 4);
    assert!(server
        .download_file(&format!("{}/api/x", api.uri()), &work.path().join("x"), 100)
        .await
        .is_ok());
}

#[tokio::test]
async fn test_download_file_redirect_answer_fails_and_is_not_followed() {
    let (mock, elsewhere) = (MockServer::start().await, MockServer::start().await);
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", format!("{}/a.xml", elsewhere.uri())))
        .mount(&mock)
        .await;
    let work = tempfile::tempdir().unwrap();

    let result = server(&mock.uri())
        .download_file(&format!("{}/a.xml", mock.uri()), &work.path().join("a.xml"), 100)
        .await;

    assert!(
        matches!(result, Err(FileError::Failed(ClientError::Status { status: 302, .. }))),
        "{result:?}"
    );
    assert!(received(&elsewhere).await.is_empty());
}

#[tokio::test]
async fn test_download_file_over_the_cap_fails_and_leaves_nothing() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("0123456789"))
        .mount(&mock)
        .await;
    let work = tempfile::tempdir().unwrap();
    let into = work.path().join("big.xml");

    let result = server(&mock.uri())
        .download_file(&format!("{}/big.xml", mock.uri()), &into, 9)
        .await;

    assert!(
        matches!(result, Err(FileError::Failed(ClientError::TooLarge { limit: 9, .. }))),
        "{result:?}"
    );
    assert!(!into.exists());
}

#[tokio::test]
async fn test_error_display_contains_no_url() {
    // Nothing listens on port 9 of the loopback address.
    let unreachable = server("http://127.0.0.1:9");
    let refused = unreachable.project_iri("0803").await.unwrap_err();
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock)
        .await;
    let status = server(&mock.uri()).export_status(IRI, "e").await.unwrap_err();
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(202).set_delay(Duration::from_secs(2)))
        .mount(&mock)
        .await;
    let timed_out = server(&mock.uri()).export(IRI).await.unwrap_err();

    for error in [refused, status, timed_out] {
        let mut shown = error.to_string();
        let mut source = std::error::Error::source(&error);
        while let Some(inner) = source {
            shown.push_str(&format!(" | {inner}"));
            source = inner.source();
        }
        assert!(!shown.contains("127.0.0.1"), "{shown}");
        assert!(!shown.contains("http://"), "{shown}");
        assert!(!shown.contains("rdfh"), "{shown}");
    }
}

#[test]
fn test_server_debug_contains_no_key_material() {
    let shown = format!("{:?}", server("https://api.example.test"));
    assert!(shown.contains("https://api.example.test"), "{shown}");
    assert!(shown.contains("test-kid-1"), "{shown}");
    let key = PRIVATE_KEY.trim();
    assert!(!shown.contains(&key[key.len() - 16..]), "{shown}");
    assert!(!shown.contains("key:"), "{shown}");
}

#[test]
fn test_server_new_refuses_a_base_that_is_not_https_or_loopback_http() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    for base in [
        "api.example.test",
        "ftp://api.example.test",
        "http://api.example.test",
        "http://localhost.example.test",
        "https://api.example.test/?x=1",
        "https://user:secret@api.example.test",
    ] {
        let identity = ServiceIdentity::new("svc", "kid", PRIVATE_KEY).unwrap();
        assert!(
            matches!(Server::new(base, identity, timeouts()), Err(ClientError::BaseUrl)),
            "{base}"
        );
    }
}

#[test]
fn test_server_new_accepts_http_on_a_loopback_host() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    for base in ["http://127.0.0.1:9", "http://localhost:9", "http://[::1]:9"] {
        let identity = ServiceIdentity::new("svc", "kid", PRIVATE_KEY).unwrap();
        assert!(Server::new(base, identity, timeouts()).is_ok(), "{base}");
    }
}
