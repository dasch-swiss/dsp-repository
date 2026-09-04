//! The Record file metadata endpoint: `GET /dpe/records/{shortcode}/{record_id}/file`.
//!
//! Exists because the technical metadata a Record's file carries — name, size,
//! checksum, dates — had no way out of DPE. See `docs/src/dpe/oai-pmh.md` →
//! *Record files* for the rationale and the response shape.

use axum::extract::Path;
use axum::response::IntoResponse;
use dpe_core::record_repository::{FsRecordRepository, RecordRepository};
use platform_metadata::RecordFile;
use serde::Serialize;

/// `Option`s serialise as explicit `null` to keep the shape stable. `path` is
/// absent entirely — assets are flat, so a null would imply a hierarchy that does
/// not exist.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileMetadata {
    file_id: String,
    file_name: Option<String>,
    download_url: String,
    file_size: Option<u64>,
    checksum: Option<String>,
    checksum_algorithm: Option<String>,
    mime_type: String,
    version: u32,
    date_created: Option<String>,
    date_modified: Option<String>,
}

/// Assets are immutable once ingested, so each is its own only version.
const ASSET_VERSION: u32 = 1;

impl FileMetadata {
    /// Every value is copied verbatim or synthesised — nothing is parsed out of
    /// `file.url`, so ingest's path shape is never matched against. That is why
    /// `file_id` is the record id and not ingest's asset id: the asset id exists
    /// only inside `file.url`, and recovering it would mean matching that shape.
    /// `date_modified` mirrors `date_created` (assets are immutable).
    fn new(file: &RecordFile, record_id: &str) -> Self {
        Self {
            file_id: record_id.to_string(),
            file_name: file.file_name.clone(),
            download_url: file.url.clone(),
            file_size: file.file_size,
            checksum: file.checksum.clone(),
            checksum_algorithm: file.checksum_algorithm.clone(),
            mime_type: file.mime_type.clone(),
            version: ASSET_VERSION,
            date_created: file.date_created.clone(),
            date_modified: file.date_created.clone(),
        }
    }
}

/// One representation, unconditionally: no `Accept` dispatch, no redirect. DPE
/// serves the metadata; the bytes come from `download_url`.
///
/// Missing record and record-without-file are both `404`, not distinguished.
/// The 404 is JSON too, not the app's HTML shell: the endpoint has no `Accept`
/// dispatch, so switching media type on the error path would hand a harvester a
/// document it cannot parse.
#[tracing::instrument(fields(otel.kind = "internal"))]
pub(crate) async fn record_file_handler(
    Path((shortcode, record_id)): Path<(String, String)>,
) -> axum::response::Response {
    let ark_suffix = format!("{shortcode}/{record_id}");
    let repo = FsRecordRepository::new();

    let Some(file) = repo.get_by_id(&ark_suffix).and_then(|r| r.file.as_ref()) else {
        return (
            axum::http::StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({ "error": "not found" })),
        )
            .into_response();
    };

    axum::Json(FileMetadata::new(file, &record_id)).into_response()
}

#[cfg(test)]
fn metadata_app(file: RecordFile) -> axum::Router {
    use std::sync::Arc;

    use axum::routing::get;

    let file = Arc::new(file);
    axum::Router::new().route(
        "/dpe/records/{shortcode}/{record_id}/file",
        get(move |Path((_, record_id)): Path<(String, String)>| {
            let file = Arc::clone(&file);
            async move { axum::Json(FileMetadata::new(&file, &record_id)).into_response() }
        }),
    )
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::header::{ACCEPT, CONTENT_TYPE, LOCATION};
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    const INGEST_URL: &str = "https://ingest.dev-03.dasch.swiss/projects/0862/assets/6YAAMJfR7sz-RWPTwYppGb7/original";
    const MINIMAL_URL: &str = "https://ingest.dasch.swiss/projects/0803/assets/lklK7rVuVOmpBZYWrF8o-g/original";
    const RECORD_ID: &str = "RMgW_EICR3OLcMi7LNE=Sgu";

    fn full_file() -> RecordFile {
        RecordFile {
            mime_type: "image/png".to_string(),
            url: INGEST_URL.to_string(),
            checksum: Some("9ab438922efe5c31f0a862e10891789d6934685bb6d146afc8a3c67c54e622c9".to_string()),
            checksum_algorithm: Some("SHA-256".to_string()),
            file_name: Some("Screenshot 2026-08-19 at 16.40.02.png".to_string()),
            file_size: Some(377685),
            date_created: Some("2026-08-25T10:25:33.455394630Z".to_string()),
        }
    }

    fn minimal_file() -> RecordFile {
        RecordFile {
            mime_type: "application/pdf".to_string(),
            url: MINIMAL_URL.to_string(),
            ..RecordFile::default()
        }
    }

    async fn get(file: RecordFile, accept: Option<&str>) -> axum::http::Response<Body> {
        let mut req = Request::builder().uri(format!("/dpe/records/0862/{RECORD_ID}/file"));
        if let Some(accept) = accept {
            req = req.header(ACCEPT, accept);
        }
        metadata_app(file).oneshot(req.body(Body::empty()).unwrap()).await.unwrap()
    }

    async fn json_body(file: RecordFile) -> serde_json::Value {
        let response = get(file, None).await;
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).expect("response body is JSON")
    }

    #[tokio::test]
    async fn returns_json_with_the_json_content_type() {
        let response = get(full_file(), None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get(CONTENT_TYPE).unwrap(), "application/json");
    }

    /// Pins the absence of content negotiation: every `Accept` gets the document.
    #[tokio::test]
    async fn accept_header_does_not_change_the_response() {
        for accept in [
            None,
            Some("*/*"),
            Some("text/html,application/xhtml+xml,*/*;q=0.8"),
            Some("application/json"),
            Some("application/xml"),
        ] {
            let response = get(full_file(), accept).await;
            assert_eq!(response.status(), StatusCode::OK, "for Accept: {accept:?}");
            assert_eq!(
                response.headers().get(CONTENT_TYPE).unwrap(),
                "application/json",
                "for Accept: {accept:?}"
            );
        }
    }

    /// The endpoint serves metadata only; it never participates in the download.
    #[tokio::test]
    async fn never_redirects() {
        for accept in [None, Some("*/*"), Some("text/html"), Some("application/json")] {
            let response = get(full_file(), accept).await;
            assert!(!response.status().is_redirection(), "for Accept: {accept:?}");
            assert!(response.headers().get(LOCATION).is_none(), "for Accept: {accept:?}");
        }
    }

    #[tokio::test]
    async fn json_body_carries_every_field_for_a_fully_populated_file() {
        let body = json_body(full_file()).await;

        assert_eq!(body["fileName"], "Screenshot 2026-08-19 at 16.40.02.png");
        assert_eq!(body["fileSize"], 377685);
        assert_eq!(
            body["checksum"],
            "9ab438922efe5c31f0a862e10891789d6934685bb6d146afc8a3c67c54e622c9"
        );
        assert_eq!(body["checksumAlgorithm"], "SHA-256");
        assert_eq!(body["mimeType"], "image/png");
        assert_eq!(body["dateCreated"], "2026-08-25T10:25:33.455394630Z");
    }

    #[tokio::test]
    async fn file_size_is_a_number() {
        let body = json_body(full_file()).await;
        assert!(
            body["fileSize"].is_u64(),
            "fileSize should be a number, got {}",
            body["fileSize"]
        );
    }

    #[tokio::test]
    async fn version_is_one_and_date_modified_mirrors_date_created() {
        let body = json_body(full_file()).await;
        assert_eq!(body["version"], 1);
        assert_eq!(body["dateModified"], body["dateCreated"]);
    }

    #[tokio::test]
    async fn download_url_is_the_ingest_url_verbatim() {
        let body = json_body(full_file()).await;
        assert_eq!(body["downloadUrl"], INGEST_URL);
    }

    /// The record id, not ingest's asset id — that one lives only inside
    /// `file.url` and recovering it would mean parsing ingest's path shape.
    #[tokio::test]
    async fn file_id_is_the_record_id() {
        let body = json_body(full_file()).await;
        assert_eq!(body["fileId"], RECORD_ID);
    }

    /// `fileId` is never derived from the URL, so the asset id must not appear.
    #[tokio::test]
    async fn file_id_is_not_the_ingest_asset_id() {
        let body = json_body(full_file()).await;
        assert_ne!(body["fileId"], "6YAAMJfR7sz-RWPTwYppGb7");
    }

    #[tokio::test]
    async fn no_path_key_is_present() {
        let body = json_body(full_file()).await;
        let object = body.as_object().expect("body is a JSON object");

        assert!(!object.contains_key("path"));
        assert!(!object.contains_key("relativePath"));
    }

    #[tokio::test]
    async fn missing_values_are_null_not_omitted() {
        let body = json_body(minimal_file()).await;
        let object = body.as_object().expect("body is a JSON object");

        for key in [
            "fileName",
            "fileSize",
            "checksum",
            "checksumAlgorithm",
            "dateCreated",
            "dateModified",
        ] {
            assert!(object.contains_key(key), "{key} should be present");
            assert!(body[key].is_null(), "{key} should be null, got {}", body[key]);
        }

        assert_eq!(body["mimeType"], "application/pdf");
        assert_eq!(body["version"], 1);
        assert_eq!(body["downloadUrl"], MINIMAL_URL);
        // Comes from the route, not the file object — never null.
        assert_eq!(body["fileId"], RECORD_ID);
    }

    /// Nothing is parsed out of the URL, so an unexpected shape changes nothing.
    #[tokio::test]
    async fn document_is_complete_for_a_url_of_an_unexpected_shape() {
        let file = RecordFile {
            url: "https://example.invalid/totally/different/layout".to_string(),
            ..full_file()
        };
        let body = json_body(file).await;

        assert_eq!(body["fileName"], "Screenshot 2026-08-19 at 16.40.02.png");
        assert_eq!(body["fileSize"], 377685);
        assert_eq!(body["mimeType"], "image/png");
        assert_eq!(body["version"], 1);
        assert_eq!(body["downloadUrl"], "https://example.invalid/totally/different/layout");
        // Not parsed out of the URL, so an unparseable one changes nothing.
        assert_eq!(body["fileId"], RECORD_ID);
    }

    #[test]
    fn date_modified_mirrors_date_created_including_when_absent() {
        let populated = FileMetadata::new(&full_file(), RECORD_ID);
        assert_eq!(populated.date_modified, populated.date_created);
        assert!(populated.date_created.is_some());

        let absent = FileMetadata::new(&minimal_file(), "lklK7rVuVOmpBZYWrF8o=gh");
        assert_eq!(absent.date_modified, absent.date_created);
        assert!(absent.date_created.is_none());
    }

    #[test]
    fn version_is_constant_regardless_of_the_file() {
        assert_eq!(FileMetadata::new(&full_file(), RECORD_ID).version, 1);
        assert_eq!(FileMetadata::new(&minimal_file(), "x").version, 1);
    }
}

/// Needs the real lookup, not the [`metadata_app`] seam. Grouped: the cache loads
/// the full dataset on first touch.
#[cfg(test)]
mod lookup_tests {
    use axum::body::Body;
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    fn app() -> axum::Router {
        dpe_core::set_data_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data"));

        axum::Router::new().route(
            "/dpe/records/{shortcode}/{record_id}/file",
            axum::routing::get(record_file_handler),
        )
    }

    async fn fetch(app: axum::Router, uri: &str) -> (StatusCode, String) {
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let response = app.oneshot(req).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    /// The endpoint speaks JSON on every path: a harvester calling it never has
    /// to parse HTML to learn the record is gone.
    #[tokio::test]
    async fn missing_record_and_record_without_a_file_both_return_a_json_404() {
        let app = app();

        let no_file_record = dpe_core::record_cache::all_records()
            .iter()
            .find(|r| r.file.is_none())
            .expect("the committed data has at least one record without a file");
        let no_file_uri = format!(
            "/dpe/records/{}/{}/file",
            no_file_record.pid.shortcode, no_file_record.pid.record_id
        );

        for uri in [
            no_file_uri.as_str(),
            "/dpe/records/0862/no-such-record/file",
            "/dpe/records/9999/no-such-record/file",
        ] {
            let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
            let response = app.clone().oneshot(req).await.unwrap();

            assert_eq!(response.status(), StatusCode::NOT_FOUND, "for {uri}");
            assert_eq!(response.headers().get(CONTENT_TYPE).unwrap(), "application/json", "for {uri}");

            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            let body: serde_json::Value =
                serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("body is JSON for {uri}: {e}"));
            assert_eq!(body["error"], "not found", "for {uri}");
        }
    }

    /// The `=` in a record id is part of the identifier — no normalisation.
    /// `%3D` is percent-decoded by Axum, so it resolves to the same record.
    #[tokio::test]
    async fn a_record_id_containing_an_equals_sign_resolves_literally_and_percent_encoded() {
        let app = app();

        let record = dpe_core::record_cache::all_records()
            .iter()
            .find(|r| r.file.is_some() && r.pid.record_id.contains('='))
            .expect("the committed data has records with files and `=` in the id");
        let expected_url = record.file.as_ref().unwrap().url.clone();

        for record_id in [record.pid.record_id.clone(), record.pid.record_id.replace('=', "%3D")] {
            let (status, body) = fetch(
                app.clone(),
                &format!("/dpe/records/{}/{}/file", record.pid.shortcode, record_id),
            )
            .await;
            assert_eq!(status, StatusCode::OK, "for {record_id}");

            let body: serde_json::Value = serde_json::from_str(&body).expect("response body is JSON");
            assert_eq!(body["downloadUrl"], expected_url, "for {record_id}");
            // Echoes the decoded segment: `%3D` comes back as a literal `=`.
            assert_eq!(body["fileId"], record.pid.record_id, "for {record_id}");
        }
    }

    #[tokio::test]
    async fn post_is_method_not_allowed() {
        let req = Request::builder()
            .method("POST")
            .uri("/dpe/records/0862/RMgW_EICR3OLcMi7LNE=Sgu/file")
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
