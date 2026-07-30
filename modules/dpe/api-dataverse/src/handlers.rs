//! Handlers for the two Dataverse-compatible endpoints.
//!
//! Both are unauthenticated: they serve published, public metadata only.

use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use dpe_core::{DataverseFileRepository, FsDataverseFileRepository};
use serde::Deserialize;

use crate::dto::VersionsResponse;
use crate::error::DataverseError;

/// Query parameters for the versions endpoint. Both are `Option` so a missing
/// `persistentId` yields our own 400 rather than Axum's rejection, matching the
/// OAI handlers' hand-rolled validation.
#[derive(Debug, Default, Deserialize)]
pub struct VersionsParams {
    #[serde(rename = "persistentId")]
    pub persistent_id: Option<String>,
    /// Sent by the crawler as `dataverse_json`. Accepted and ignored: it selects
    /// an exporter in real Dataverse, and this endpoint only speaks that one
    /// format, so it cannot change the response.
    pub exporter: Option<String>,
}

/// `GET /api/datasets/:persistentId/versions/:latest-published`
///
/// The path segments containing colons are literal (see the route table); the
/// dataset is selected by the `persistentId` query parameter instead.
#[axum::debug_handler]
#[tracing::instrument(skip_all, fields(otel.kind = "internal", oai.identifier))]
pub async fn versions_handler(Query(params): Query<VersionsParams>) -> Response {
    match resolve_versions(&params, &FsDataverseFileRepository::new()) {
        Ok(response) => Json(response).into_response(),
        Err(err) => err.into_response(),
    }
}

/// Resolve the versions response for the given params. Pure over its repository so
/// the contract is testable without the process caches.
pub(crate) fn resolve_versions(
    params: &VersionsParams,
    files: &dyn DataverseFileRepository,
) -> Result<VersionsResponse, DataverseError> {
    let identifier = params
        .persistent_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or(DataverseError::MissingPersistentId)?;

    tracing::Span::current().record("oai.identifier", identifier);

    // `files_for` distinguishes the two negative cases: `Some(vec![])` is a record
    // we hold that has no files — the majority, roughly 77% of records — and is
    // answered with an empty list, which the crawler reads as a dataset without
    // files. `None` is an identifier we do not know at all, and is a 404, so a
    // typo'd identifier is not silently indistinguishable from a metadata-only
    // record.
    files
        .files_for(identifier)
        .map(VersionsResponse::ok)
        .ok_or_else(|| DataverseError::DatasetNotFound(identifier.to_string()))
}

/// `GET /api/access/datafile/{id}`
///
/// A non-numeric segment fails in the extractor before reaching here.
#[tracing::instrument(skip_all, fields(otel.kind = "internal", file.id = id))]
pub async fn datafile_handler(Path(id): Path<u64>) -> Response {
    match resolve_datafile(id, &FsDataverseFileRepository::new()) {
        Ok(url) => Redirect::temporary(&url).into_response(),
        Err(err) => err.into_response(),
    }
}

/// Resolve the download target for a file id, or the error to return.
///
/// Redirecting rather than proxying is what production Dataverse does (Harvard
/// answers with a 303 to a presigned S3 URL), so clients already follow it.
/// `Content-Disposition` deliberately is not set here: it belongs on the final
/// hop, which dsp-ingest serves.
pub(crate) fn resolve_datafile(id: u64, files: &dyn DataverseFileRepository) -> Result<String, DataverseError> {
    let file = files.file_by_id(id).ok_or(DataverseError::FileNotFound(id))?;

    // Currently unreachable: `DataverseFile::RESTRICTED` is a constant `false`
    // because all DaSCH data is open. Kept because it is the correct response the
    // moment a real restriction signal exists, and because deleting it would leave
    // the contract's `restricted` semantics implemented nowhere.
    if file.restricted {
        return Err(DataverseError::FileRestricted(id));
    }

    Ok(file.download_url)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::http::StatusCode;
    use dpe_core::{file_id_for_iri, DataverseFile, FilePlaceholders};

    use super::*;

    // ---- fakes ----

    /// In-memory stand-in for the record-backed repository.
    ///
    /// Distinguishes the two negative cases the same way the production
    /// implementation does: a key present with an empty vec is a known record with
    /// no files; a key absent is an unknown identifier.
    struct InMemoryFiles {
        by_identifier: HashMap<String, Vec<DataverseFile>>,
    }

    impl InMemoryFiles {
        fn new(entries: Vec<(&str, Vec<DataverseFile>)>) -> Self {
            Self {
                by_identifier: entries.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            }
        }

        fn empty() -> Self {
            Self { by_identifier: HashMap::new() }
        }
    }

    impl DataverseFileRepository for InMemoryFiles {
        fn files_for(&self, oai_identifier: &str) -> Option<Vec<DataverseFile>> {
            self.by_identifier.get(oai_identifier).cloned()
        }

        fn file_by_id(&self, id: u64) -> Option<DataverseFile> {
            self.by_identifier.values().flatten().find(|f| f.id == id).cloned()
        }
    }

    // ---- fixtures ----

    const RECORD_ID: &str = "oai:dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh";
    const RECORD_IRI: &str = "http://rdfh.ch/0803/lklK7rVuVOmpBZYWrF8o-g";
    const ASSET_URL: &str = "https://ingest.dasch.swiss/projects/0803/assets/2vbIabBOEvq-EU9jwmgEe9j/original";

    /// A file as `DataverseFile::from_record` would build it: real MIME type, date
    /// and URL; placeholder filename, size and checksum.
    fn jp2_file() -> DataverseFile {
        DataverseFile {
            id: file_id_for_iri(RECORD_IRI),
            content_type: "image/jp2".to_string(),
            creation_date: "2012-06-19T14:33:33Z".to_string(),
            restricted: false,
            download_url: ASSET_URL.to_string(),
            placeholders: FilePlaceholders::for_asset("2vbIabBOEvq-EU9jwmgEe9j", "image/jp2"),
        }
    }

    /// A second file whose MIME type carries parameters, to pin that they survive.
    fn csv_file() -> DataverseFile {
        DataverseFile {
            id: file_id_for_iri("http://rdfh.ch/0868/other"),
            content_type: "text/csv; charset=UTF-8".to_string(),
            creation_date: "2019-06-30".to_string(),
            restricted: false,
            download_url: "https://ingest.dasch.swiss/projects/0868/assets/abc/original".to_string(),
            placeholders: FilePlaceholders::for_asset("abc", "text/csv; charset=UTF-8"),
        }
    }

    fn params(persistent_id: Option<&str>, exporter: Option<&str>) -> VersionsParams {
        VersionsParams {
            persistent_id: persistent_id.map(str::to_string),
            exporter: exporter.map(str::to_string),
        }
    }

    fn body_of(response: &VersionsResponse) -> serde_json::Value {
        serde_json::to_value(response).expect("response should serialize")
    }

    fn one_file_table() -> InMemoryFiles {
        InMemoryFiles::new(vec![(RECORD_ID, vec![jp2_file()])])
    }

    // ---- contract: the response envelope ----

    #[test]
    fn golden_envelope_for_a_record_with_one_file() {
        // The full contract shape, asserted exactly: nesting, key names, and the
        // sibling/member split between `restricted`/`version` and `dataFile`.
        let response = resolve_versions(&params(Some(RECORD_ID), Some("dataverse_json")), &one_file_table())
            .expect("known record should resolve");

        let expected = serde_json::json!({
            "status": "OK",
            "data": {
                "files": [
                    {
                        "restricted": false,
                        "version": 1,
                        "dataFile": {
                            "id": file_id_for_iri(RECORD_IRI),
                            "filename": "2vbIabBOEvq-EU9jwmgEe9j.jp2",
                            "contentType": "image/jp2",
                            "filesize": 1,
                            "creationDate": "2012-06-19T14:33:33Z",
                            "checksum": { "type": "MD5", "value": "d41d8cd98f00b204e9800998ecf8427e" }
                        }
                    }
                ]
            }
        });

        assert_eq!(body_of(&response), expected);
    }

    #[test]
    fn absent_optionals_are_omitted_not_null() {
        // A `null` where the parser expects a string is a different shape than a
        // missing key, so this distinction is contractual, not cosmetic. DPE has no
        // per-file modification date, so `lastUpdateTime` is always absent; and
        // `directoryLabel` is never emitted at all.
        let response =
            resolve_versions(&params(Some(RECORD_ID), None), &one_file_table()).expect("known record should resolve");
        let body = body_of(&response);
        let entry = &body["data"]["files"][0];

        let entry_keys: Vec<&String> = entry.as_object().expect("file entry is an object").keys().collect();
        let data_file_keys: Vec<&String> =
            entry["dataFile"].as_object().expect("dataFile is an object").keys().collect();

        assert!(!entry_keys.contains(&&"directoryLabel".to_string()));
        assert!(!data_file_keys.contains(&&"lastUpdateTime".to_string()));
    }

    #[test]
    fn every_required_field_is_present_on_every_file() {
        // A loop rather than per-field assertions, so adding a file cannot silently
        // regress a field the parser hard-fails on.
        let response = resolve_versions(
            &params(Some(RECORD_ID), None),
            &InMemoryFiles::new(vec![(RECORD_ID, vec![jp2_file(), csv_file()])]),
        )
        .expect("known record should resolve");
        let body = body_of(&response);
        let files = body["data"]["files"].as_array().expect("files is an array");

        assert_eq!(files.len(), 2);
        for file in files {
            assert!(file["restricted"].is_boolean(), "restricted missing on {file}");
            assert!(file["version"].is_u64(), "version missing on {file}");

            let data_file = &file["dataFile"];
            assert!(data_file["id"].is_u64(), "dataFile.id missing on {file}");
            assert!(data_file["filename"].is_string(), "dataFile.filename missing on {file}");
            assert!(data_file["contentType"].is_string(), "dataFile.contentType missing on {file}");
            assert!(data_file["filesize"].is_u64(), "dataFile.filesize missing on {file}");
            assert!(data_file["creationDate"].is_string(), "dataFile.creationDate missing on {file}");
            assert!(data_file["checksum"]["type"].is_string(), "checksum.type missing on {file}");
            assert!(data_file["checksum"]["value"].is_string(), "checksum.value missing on {file}");
        }
    }

    #[test]
    fn file_ids_stay_within_json_safe_integer_range() {
        // Consumers that parse JSON numbers as doubles round above 2^53, which would
        // corrupt the download URL derived from the id.
        let response = resolve_versions(
            &params(Some(RECORD_ID), None),
            &InMemoryFiles::new(vec![(RECORD_ID, vec![jp2_file(), csv_file()])]),
        )
        .expect("known record should resolve");
        let body = body_of(&response);

        for file in body["data"]["files"].as_array().expect("files is an array") {
            let id = file["dataFile"]["id"].as_u64().expect("id is numeric");
            assert!(id < (1 << 53), "id {id} exceeds the JSON-safe integer range");
        }
    }

    #[test]
    fn mime_parameters_survive_unaltered() {
        let response = resolve_versions(
            &params(Some(RECORD_ID), None),
            &InMemoryFiles::new(vec![(RECORD_ID, vec![csv_file()])]),
        )
        .expect("known record should resolve");

        assert_eq!(
            body_of(&response)["data"]["files"][0]["dataFile"]["contentType"],
            "text/csv; charset=UTF-8"
        );
    }

    #[test]
    fn exporter_param_does_not_change_the_body() {
        let with = resolve_versions(&params(Some(RECORD_ID), Some("dataverse_json")), &one_file_table())
            .expect("known record should resolve");
        let without =
            resolve_versions(&params(Some(RECORD_ID), None), &one_file_table()).expect("known record should resolve");

        assert_eq!(body_of(&with), body_of(&without));
    }

    #[test]
    fn identifier_with_ark_colons_is_accepted_verbatim() {
        // The crawler passes the OAI header identifier unmodified, colons and all.
        // Anything that split on ':' would mangle the ARK and miss the record.
        assert!(
            resolve_versions(&params(Some(RECORD_ID), None), &one_file_table()).is_ok(),
            "full OAI identifier form should resolve"
        );
    }

    // ---- contract: errors and the empty-list path ----

    #[test]
    fn missing_persistent_id_is_bad_request() {
        let err =
            resolve_versions(&params(None, None), &one_file_table()).expect_err("missing persistentId should fail");

        assert_eq!(err, DataverseError::MissingPersistentId);
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn blank_persistent_id_is_bad_request() {
        let err =
            resolve_versions(&params(Some("  "), None), &one_file_table()).expect_err("blank persistentId should fail");

        assert_eq!(err, DataverseError::MissingPersistentId);
    }

    #[test]
    fn unknown_identifier_is_not_found() {
        let err = resolve_versions(
            &params(Some("oai:dasch.swiss:ark:/72163/1/9999"), None),
            &InMemoryFiles::empty(),
        )
        .expect_err("unknown identifier should fail");

        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn known_record_without_files_yields_empty_list() {
        // The dominant case: ~77% of records are metadata-only. An empty array is
        // distinguishable from the 404 an unknown identifier gets, and is what keeps
        // a harvest of mostly-fileless records from looking like a broken endpoint.
        let response = resolve_versions(&params(Some(RECORD_ID), None), &InMemoryFiles::new(vec![(RECORD_ID, vec![])]))
            .expect("known record should resolve");

        let body = body_of(&response);
        assert_eq!(body["status"], "OK");
        assert_eq!(body["data"]["files"].as_array().expect("files is an array").len(), 0);
    }

    // ---- download endpoint ----

    #[test]
    fn open_file_resolves_to_its_download_url() {
        let url = resolve_datafile(file_id_for_iri(RECORD_IRI), &one_file_table()).expect("open file should resolve");

        assert_eq!(url, ASSET_URL);
    }

    #[test]
    fn unknown_file_id_is_not_found() {
        let err = resolve_datafile(999_999, &one_file_table()).expect_err("unknown id should fail");

        assert_eq!(err, DataverseError::FileNotFound(999_999));
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn restricted_file_is_forbidden() {
        // Unreachable with real data today (every file is open), but the branch must
        // stay correct for when a restriction signal exists.
        let mut restricted = jp2_file();
        restricted.restricted = true;
        let table = InMemoryFiles::new(vec![(RECORD_ID, vec![restricted])]);

        let err = resolve_datafile(file_id_for_iri(RECORD_IRI), &table).expect_err("restricted file should fail");

        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }
}
