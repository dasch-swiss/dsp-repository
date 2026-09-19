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
// All dump endpoints require a system-admin bearer token. Tests assert:
//   1. The correct URL path + query is requested.
//   2. `Authorization: Bearer <token>` header is sent.
//   3. Responses are correctly mapped to domain types or Diagnostics.
//   4. Content-Disposition from the download endpoint is intentionally ignored.

use std::io::{self, Write};

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::{CreateDumpOutcome, DumpStatus};
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// FailingWriter — always returns an io::Error on write
// ---------------------------------------------------------------------------

/// A `Write` sink whose `write` always returns `Err(ErrorKind::Other)`.
///
/// Used in `download_write_failure_returns_io_not_network` to verify that
/// disk-write failures are classified as `Diagnostic::Io`, not `Network`.
struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("simulated disk full"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

const TOKEN: &str = "test-system-admin-token";
const PROJECT_IRI: &str = "http://rdfh.ch/projects/0001";
// URL-encoded form of PROJECT_IRI with NON_ALPHANUMERIC (same set used by enc()).
// Matches: http%3A%2F%2Frdfh%2Ech%2Fprojects%2F0001
const ENCODED_IRI: &str = "http%3A%2F%2Frdfh%2Ech%2Fprojects%2F0001";
const DUMP_ID: &str = "dGVzdC1pZA"; // URL-safe base64; inserted verbatim

/// Minimal DataTaskStatusResponse body for the given status string.
fn task_body(id: &str, status: &str) -> serde_json::Value {
    json!({ "id": id, "status": status })
}

// ---------------------------------------------------------------------------
// create_project_dump — happy path (trigger)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn trigger_happy_returns_in_progress_task() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports");
    let mock = Mock::given(method("POST"))
        .and(path(expected_path))
        .and(query_param("skipAssets", "false"))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(202).set_body_json(task_body("task-1", "in_progress")))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.create_project_dump(&uri, PROJECT_IRI, false, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let outcome = result.unwrap();
    assert!(
        matches!(outcome, CreateDumpOutcome::Created(_)),
        "202 response must yield Created, got: {outcome:?}"
    );
    if let CreateDumpOutcome::Created(task) = outcome {
        assert_eq!(task.id, "task-1");
        assert_eq!(task.status, DumpStatus::InProgress);
    }

    drop(mock);
}

#[tokio::test]
async fn trigger_sends_bearer_header() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports");
    Mock::given(method("POST"))
        .and(path(expected_path))
        .respond_with(ResponseTemplate::new(202).set_body_json(task_body("task-1", "in_progress")))
        .mount(&server)
        .await;

    let uri = server.uri();
    let _ = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.create_project_dump(&uri, PROJECT_IRI, false, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    let received = server.received_requests().await.expect("request recording should be enabled");
    assert_eq!(received.len(), 1);
    let auth = received[0]
        .headers
        .get("authorization")
        .expect("Authorization header must be present for dump trigger");
    assert_eq!(
        auth.to_str().expect("header should be valid UTF-8"),
        format!("Bearer {TOKEN}"),
        "bearer token must match"
    );
}

#[tokio::test]
async fn trigger_skip_assets_true_sends_correct_query() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports");
    let mock = Mock::given(method("POST"))
        .and(path(expected_path))
        .and(query_param("skipAssets", "true"))
        .respond_with(ResponseTemplate::new(202).set_body_json(task_body("task-skip", "in_progress")))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.create_project_dump(&uri, PROJECT_IRI, true, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok with skipAssets=true");
    drop(mock);
}

/// 409 with the real `export_exists` conflict body where projectIri == request IRI
/// → same-project case → `Exists { id }`.
#[tokio::test]
async fn trigger_409_export_exists_body_returns_exists_outcome() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports");
    let conflict_body = json!({
        "errors": [{
            "code": "export_exists",
            "details": {
                "id": DUMP_ID,
                // projectIri == PROJECT_IRI → same-project → Exists (not ExistsForOtherProject)
                "projectIri": PROJECT_IRI
            }
        }]
    });
    Mock::given(method("POST"))
        .and(path(expected_path))
        .respond_with(ResponseTemplate::new(409).set_body_json(conflict_body))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.create_project_dump(&uri, PROJECT_IRI, false, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok(Exists), got: {:?}", result);
    let outcome = result.unwrap();
    match outcome {
        CreateDumpOutcome::Exists { id } => {
            assert_eq!(id, DUMP_ID, "id must match the one in the conflict body");
            // Explicitly assert this is Exists (same project), NOT ExistsForOtherProject.
        }
        CreateDumpOutcome::ExistsForOtherProject { .. } => {
            panic!("same-project 409 must yield Exists, not ExistsForOtherProject")
        }
        other => panic!("expected Exists, got: {other:?}"),
    }
}

/// 409 with `export_exists` where projectIri differs from request IRI
/// → cross-project case → `ExistsForOtherProject { id, project_iri }`.
#[tokio::test]
async fn trigger_409_export_exists_other_project_returns_other_project_outcome() {
    let server = MockServer::start().await;

    const OTHER_PROJECT_IRI: &str = "http://rdfh.ch/projects/0002";

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports");
    let conflict_body = json!({
        "errors": [{
            "code": "export_exists",
            "details": {
                "id": DUMP_ID,
                // projectIri DIFFERS from the requested PROJECT_IRI → ExistsForOtherProject
                "projectIri": OTHER_PROJECT_IRI
            }
        }]
    });
    Mock::given(method("POST"))
        .and(path(expected_path))
        .respond_with(ResponseTemplate::new(409).set_body_json(conflict_body))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.create_project_dump(&uri, PROJECT_IRI, false, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok(ExistsForOtherProject), got: {:?}", result);
    let outcome = result.unwrap();
    match outcome {
        CreateDumpOutcome::ExistsForOtherProject { id, project_iri } => {
            assert_eq!(id, DUMP_ID, "id must match the conflict body");
            assert_eq!(
                project_iri, OTHER_PROJECT_IRI,
                "project_iri must be the occupying project (not the requested one)"
            );
        }
        other => panic!("expected ExistsForOtherProject, got: {other:?}"),
    }
}

/// 409 with `export_exists` but the `projectIri` field is absent → FAIL CLOSED → `ServerError`.
///
/// The 007 PRD documents `projectIri` as always present; its absence is a server-contract
/// violation. We must not silently adopt the dump (that re-enables the data-confusion bug).
#[tokio::test]
async fn trigger_409_export_exists_missing_project_iri_is_server_error() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports");
    let conflict_body = json!({
        "errors": [{
            "code": "export_exists",
            "details": {
                "id": DUMP_ID
                // "projectIri" key deliberately absent → fail-closed
            }
        }]
    });
    Mock::given(method("POST"))
        .and(path(expected_path))
        .respond_with(ResponseTemplate::new(409).set_body_json(conflict_body))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.create_project_dump(&uri, PROJECT_IRI, false, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "missing projectIri must yield an error (fail-closed)");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "missing projectIri must map to ServerError (fail-closed, not Exists)"
    );
}

/// 409 with a different error code (not `export_exists`) → `ServerError`.
#[tokio::test]
async fn trigger_409_other_code_returns_server_error() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports");
    let conflict_body = json!({
        "errors": [{
            "code": "some_other_error",
            "details": {}
        }]
    });
    Mock::given(method("POST"))
        .and(path(expected_path))
        .respond_with(ResponseTemplate::new(409).set_body_json(conflict_body))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.create_project_dump(&uri, PROJECT_IRI, false, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 409 with unknown code");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "409 with non-export_exists code must map to ServerError"
    );
}

/// 409 with an unparseable / empty body → `ServerError`.
#[tokio::test]
async fn trigger_409_empty_body_returns_server_error() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports");
    Mock::given(method("POST"))
        .and(path(expected_path))
        .respond_with(ResponseTemplate::new(409).set_body_string(""))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.create_project_dump(&uri, PROJECT_IRI, false, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 409 with empty body");
    let err = result.unwrap_err();
    assert!(
        matches!(err, Diagnostic::ServerError(_)),
        "409 with empty body must map to ServerError"
    );
    // dsp-cli/ADR-0001: user-facing error messages must not contain the DSP-API word "export".
    if let Diagnostic::ServerError(msg) = &err {
        assert!(
            !msg.to_lowercase().contains("export"),
            "ServerError message must not contain 'export' (dsp-cli/ADR-0001): {msg}"
        );
    }
}

/// 409 with a body that parses as V3ErrorBody but has no `id` in details → `ServerError`.
#[tokio::test]
async fn trigger_409_export_exists_missing_id_returns_server_error() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports");
    let conflict_body = json!({
        "errors": [{
            "code": "export_exists",
            "details": {
                "projectIri": PROJECT_IRI
                // "id" key deliberately absent
            }
        }]
    });
    Mock::given(method("POST"))
        .and(path(expected_path))
        .respond_with(ResponseTemplate::new(409).set_body_json(conflict_body))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.create_project_dump(&uri, PROJECT_IRI, false, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 409 with no id in details");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::ServerError(_)),
        "409 export_exists with missing id must map to ServerError"
    );
}

/// 409 with `export_exists` and a valid `createdAt` in the 202 response
/// → `created_at` is parsed from the status endpoint (not the trigger body).
/// This test covers the `created_at` field in a full 202 trigger response.
#[tokio::test]
async fn trigger_202_with_created_at_populates_field() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports");
    let body_with_ts = json!({
        "id": "task-ts",
        "status": "in_progress",
        "createdAt": "2026-05-20T14:03:00Z"
    });
    let mock = Mock::given(method("POST"))
        .and(path(expected_path))
        .respond_with(ResponseTemplate::new(202).set_body_json(body_with_ts))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.create_project_dump(&uri, PROJECT_IRI, false, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    if let Ok(CreateDumpOutcome::Created(task)) = result {
        let ts = task.created_at.expect("created_at should be Some for this response");
        use chrono::Datelike;
        assert_eq!(ts.year(), 2026);
        assert_eq!(ts.month(), 5);
        assert_eq!(ts.day(), 20);
    } else {
        panic!("expected Created outcome");
    }
    drop(mock);
}

#[tokio::test]
async fn trigger_401_returns_auth_required() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports");
    Mock::given(method("POST"))
        .and(path(expected_path))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.create_project_dump(&uri, PROJECT_IRI, false, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 401");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::AuthRequired(_)),
        "401 must map to Diagnostic::AuthRequired"
    );
}

// ---------------------------------------------------------------------------
// get_project_dump_status — poll
// ---------------------------------------------------------------------------

#[tokio::test]
async fn poll_200_in_progress_returns_correct_status() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}");
    let mock = Mock::given(method("GET"))
        .and(path(&expected_path))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(task_body(DUMP_ID, "in_progress")))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.get_project_dump_status(&uri, PROJECT_IRI, DUMP_ID, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    assert_eq!(result.unwrap().status, DumpStatus::InProgress);
    drop(mock);
}

#[tokio::test]
async fn poll_200_completed_returns_correct_status() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}");
    let mock = Mock::given(method("GET"))
        .and(path(&expected_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(task_body(DUMP_ID, "completed")))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.get_project_dump_status(&uri, PROJECT_IRI, DUMP_ID, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    assert_eq!(result.unwrap().status, DumpStatus::Completed);
    drop(mock);
}

#[tokio::test]
async fn poll_transitions_in_progress_then_completed() {
    // Mount two sequential mocks. The first is exhausted after one match,
    // so the second call hits the second mock. This exercises the poll
    // progression pattern (in_progress → completed).
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}");

    // First call → in_progress
    Mock::given(method("GET"))
        .and(path(&expected_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(task_body(DUMP_ID, "in_progress")))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second call → completed
    Mock::given(method("GET"))
        .and(path(&expected_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(task_body(DUMP_ID, "completed")))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result1 = {
        let uri_c = uri.clone();
        std::thread::spawn(move || {
            let client = HttpDspClient::new().expect("client construction should not fail");
            client.get_project_dump_status(&uri_c, PROJECT_IRI, DUMP_ID, TOKEN)
        })
        .join()
        .expect("blocking thread should not panic")
    };

    let result2 = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.get_project_dump_status(&uri, PROJECT_IRI, DUMP_ID, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert_eq!(result1.unwrap().status, DumpStatus::InProgress);
    assert_eq!(result2.unwrap().status, DumpStatus::Completed);
}

#[tokio::test]
async fn poll_404_returns_not_found() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}");
    Mock::given(method("GET"))
        .and(path(&expected_path))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.get_project_dump_status(&uri, PROJECT_IRI, DUMP_ID, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 404");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::NotFound(_)),
        "404 must map to Diagnostic::NotFound"
    );
}

// ---------------------------------------------------------------------------
// download_project_dump — happy path + Content-Disposition ignore
// ---------------------------------------------------------------------------

#[tokio::test]
async fn download_happy_streams_bytes_and_ignores_content_disposition() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}/download");
    let zip_bytes: Vec<u8> = b"PK\x03\x04hello-zip-content".to_vec();
    let zip_bytes_clone = zip_bytes.clone();

    // The server deliberately sends a Content-Disposition header — the client
    // must ignore it and write bytes verbatim to the dest sink (the action owns
    // the filename, not the server).
    Mock::given(method("GET"))
        .and(path(&expected_path))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(zip_bytes.clone())
                .insert_header("Content-Disposition", "attachment; filename=\"server-chosen-name.zip\"")
                .insert_header("Content-Type", "application/zip"),
        )
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        let mut sink: Vec<u8> = Vec::new();
        let count = client.download_project_dump(&uri, PROJECT_IRI, DUMP_ID, TOKEN, &mut sink)?;
        Ok::<(Vec<u8>, u64), Diagnostic>((sink, count))
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    let (sink_bytes, count) = result.unwrap();

    // Bytes land correctly in the sink.
    assert_eq!(sink_bytes, zip_bytes_clone, "downloaded bytes must match the server body");
    assert_eq!(
        count,
        zip_bytes_clone.len() as u64,
        "returned byte count must match body length"
    );

    // The server sent a Content-Disposition header — assert it was ignored by
    // checking the sink content equals the raw body bytes (not a renamed file).
    // (Structural: if Content-Disposition were honoured, behaviour would differ;
    // this test documents the intentional no-honour as per the plan spec.)
}

#[tokio::test]
async fn download_409_returns_conflict() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}/download");
    Mock::given(method("GET"))
        .and(path(&expected_path))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(409))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        let mut sink: Vec<u8> = Vec::new();
        client.download_project_dump(&uri, PROJECT_IRI, DUMP_ID, TOKEN, &mut sink)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 409 download");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::Conflict(_)),
        "download 409 must map to Diagnostic::Conflict"
    );
}

#[tokio::test]
async fn download_404_returns_not_found() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}/download");
    Mock::given(method("GET"))
        .and(path(&expected_path))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        let mut sink: Vec<u8> = Vec::new();
        client.download_project_dump(&uri, PROJECT_IRI, DUMP_ID, TOKEN, &mut sink)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 404 download");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::NotFound(_)),
        "download 404 must map to Diagnostic::NotFound"
    );
}

#[tokio::test]
async fn download_401_returns_auth_required() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}/download");
    Mock::given(method("GET"))
        .and(path(&expected_path))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        let mut sink: Vec<u8> = Vec::new();
        client.download_project_dump(&uri, PROJECT_IRI, DUMP_ID, TOKEN, &mut sink)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 401 download");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::AuthRequired(_)),
        "download 401 must map to Diagnostic::AuthRequired"
    );
}

// ---------------------------------------------------------------------------
// delete_project_dump
// ---------------------------------------------------------------------------

#[tokio::test]
async fn delete_204_returns_ok() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}");
    let mock = Mock::given(method("DELETE"))
        .and(path(&expected_path))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount_as_scoped(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.delete_project_dump(&uri, PROJECT_IRI, DUMP_ID, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_ok(), "expected Ok for 204 delete, got: {:?}", result);
    drop(mock);
}

#[tokio::test]
async fn delete_409_returns_conflict() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}");
    Mock::given(method("DELETE"))
        .and(path(&expected_path))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(409))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.delete_project_dump(&uri, PROJECT_IRI, DUMP_ID, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 409 delete");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::Conflict(_)),
        "delete 409 must map to Diagnostic::Conflict"
    );
}

#[tokio::test]
async fn delete_404_returns_not_found() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}");
    Mock::given(method("DELETE"))
        .and(path(&expected_path))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.delete_project_dump(&uri, PROJECT_IRI, DUMP_ID, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 404 delete");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::NotFound(_)),
        "delete 404 must map to Diagnostic::NotFound"
    );
}

#[tokio::test]
async fn delete_401_returns_auth_required() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}");
    Mock::given(method("DELETE"))
        .and(path(&expected_path))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        client.delete_project_dump(&uri, PROJECT_IRI, DUMP_ID, TOKEN)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err for 401 delete");
    assert!(
        matches!(result.unwrap_err(), Diagnostic::AuthRequired(_)),
        "delete 401 must map to Diagnostic::AuthRequired"
    );
}

// ---------------------------------------------------------------------------
// Fix 1 regression: download write-failure → Diagnostic::Io (not Network)
// ---------------------------------------------------------------------------

/// Regression test for the `io::copy` replacement (Fix 1, review).
///
/// The mock server returns 200 + a body. The `dest` sink is a `FailingWriter`
/// that always errors on `write`. The test asserts the result is
/// `Err(Diagnostic::Io(_))` — NOT `Network` — proving that disk-write
/// failures are distinguished from network read failures.
#[tokio::test]
async fn download_write_failure_returns_io_not_network() {
    let server = MockServer::start().await;

    let expected_path = format!("/v3/projects/{ENCODED_IRI}/exports/{DUMP_ID}/download");
    Mock::given(method("GET"))
        .and(path(&expected_path))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"PK\x03\x04some-zip-content".to_vec())
                .insert_header("Content-Type", "application/zip"),
        )
        .mount(&server)
        .await;

    let uri = server.uri();
    let result = std::thread::spawn(move || {
        let client = HttpDspClient::new().expect("client construction should not fail");
        let mut sink = FailingWriter;
        client.download_project_dump(&uri, PROJECT_IRI, DUMP_ID, TOKEN, &mut sink)
    })
    .join()
    .expect("blocking thread should not panic");

    assert!(result.is_err(), "expected Err when dest.write_all fails");
    let err = result.unwrap_err();
    assert!(
        matches!(err, Diagnostic::Io(_)),
        "a dest write failure must be Diagnostic::Io, not Network; got: {err:?}"
    );
}
