//! Handler for the OAI-PMH GetRecord verb.

use dpe_core::{Project, ProjectRepository, Record, RecordRepository};

use super::{build_error_response, OaiParams, SUPPORTED_PREFIXES};
use crate::error::OaiError;
use crate::metadata::{parse_oai_identifier, to_oai_record, to_oai_record_from_record, OaiRecord};
use crate::xml::OaiXmlBuilder;

/// Handles the GetRecord verb.
pub fn handle_get_record(
    params: &OaiParams,
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
) -> String {
    let result = require_identifier(params)
        .and_then(|id| require_metadata_prefix(params).map(|prefix| (id, prefix)))
        .and_then(|(id, prefix)| reject_unexpected_args(params).map(|_| (id, prefix)))
        .and_then(|(id, prefix)| resolve_entity(id, repo, record_repo).map(|oai| (id, prefix, oai)))
        .map(|(id, prefix, oai)| build_response(id, prefix, oai));

    result.unwrap_or_else(|err| build_error_response(err, Some("GetRecord")))
}

fn require_identifier(params: &OaiParams) -> Result<&str, OaiError> {
    params
        .identifier
        .as_deref()
        .ok_or_else(|| OaiError::BadArgument("identifier argument is required".to_string()))
}

fn require_metadata_prefix(params: &OaiParams) -> Result<&str, OaiError> {
    let prefix = params
        .metadata_prefix
        .as_deref()
        .ok_or_else(|| OaiError::BadArgument("metadataPrefix argument is required".to_string()))?;

    if !SUPPORTED_PREFIXES.contains(&prefix) {
        return Err(OaiError::CannotDisseminateFormat);
    }

    Ok(prefix)
}

fn reject_unexpected_args(params: &OaiParams) -> Result<(), OaiError> {
    if params.from.is_some() || params.until.is_some() || params.set.is_some() || params.resumption_token.is_some() {
        return Err(OaiError::BadArgument("Unexpected argument for GetRecord".to_string()));
    }
    Ok(())
}

enum OaiEntity {
    Project(Box<Project>),
    Record(Box<Record>),
}

/// Resolves an OAI identifier against both the project and record repositories.
/// Tries project repo first; falls back to record repo.
fn resolve_entity(
    identifier: &str,
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
) -> Result<OaiEntity, OaiError> {
    let id = parse_oai_identifier(identifier).ok_or(OaiError::IdDoesNotExist)?;

    if let Some(project) = repo.get_by_shortcode(&id) {
        return Ok(OaiEntity::Project(Box::new(project.clone())));
    }

    if let Some(record) = record_repo.get_by_id(&id) {
        return Ok(OaiEntity::Record(Box::new(record.clone())));
    }

    Err(OaiError::IdDoesNotExist)
}

fn build_response(identifier: &str, prefix: &str, entity: OaiEntity) -> String {
    let oai_record: OaiRecord = match entity {
        OaiEntity::Project(ref project) => to_oai_record(project, prefix),
        OaiEntity::Record(ref record) => to_oai_record_from_record(record, prefix),
    };

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("GetRecord", &[("identifier", identifier), ("metadataPrefix", prefix)]);
    builder.start_element("GetRecord");
    builder.write_record(&oai_record);
    builder.end_element("GetRecord");
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::test_utils::{
        golden, incunabula_project, normalize, InMemoryProjectRepository, InMemoryRecordRepository,
    };
    use dpe_core::Record;

    fn first_0803_record() -> Record {
        let json = include_str!("../../../server/data/records/0803-records.json");
        let [record]: [Record; 1] = serde_json::from_str(json).expect("parse 0803-records.json");
        record
    }

    fn make_params(identifier: Option<&str>, metadata_prefix: Option<&str>) -> OaiParams {
        OaiParams {
            verb: Some("GetRecord".to_string()),
            identifier: identifier.map(str::to_string),
            metadata_prefix: metadata_prefix.map(str::to_string),
            from: None,
            until: None,
            set: None,
            resumption_token: None,
        }
    }

    fn repo_with_incunabula() -> InMemoryProjectRepository {
        InMemoryProjectRepository::new(vec![incunabula_project()])
    }

    fn repo_with_record() -> InMemoryRecordRepository {
        InMemoryRecordRepository::new(vec![first_0803_record()])
    }

    // ---- error cases ----

    #[test]
    fn missing_identifier_returns_bad_argument() {
        let params = make_params(None, Some("oai_dc"));
        let xml = handle_get_record(&params, &repo_with_incunabula(), &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("identifier argument is required"), "got: {}", xml);
        assert!(
            xml.contains("verb=\"GetRecord\""),
            "verb should be echoed in request element, got: {}",
            xml
        );
    }

    #[test]
    fn missing_metadata_prefix_returns_bad_argument() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/0803"), None);
        let xml = handle_get_record(&params, &repo_with_incunabula(), &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("metadataPrefix argument is required"), "got: {}", xml);
    }

    #[test]
    fn unsupported_metadata_prefix_returns_cannot_disseminate() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/0803"), Some("marc21"));
        let xml = handle_get_record(&params, &repo_with_incunabula(), &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"cannotDisseminateFormat\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_id_not_in_either_repo_returns_id_does_not_exist() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/9999"), Some("oai_dc"));
        let xml = handle_get_record(&params, &repo_with_incunabula(), &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"idDoesNotExist\">"), "got: {}", xml);
    }

    #[test]
    fn unexpected_argument_returns_bad_argument() {
        let mut params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/0803"), Some("oai_dc"));
        params.set = Some("entityType:ResearchProject".to_string());
        let xml = handle_get_record(&params, &repo_with_incunabula(), &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    // ---- project golden tests (existing behaviour) ----

    #[test]
    fn golden_oai_dc_response() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/0803"), Some("oai_dc"));
        let xml = handle_get_record(&params, &repo_with_incunabula(), &InMemoryRecordRepository::empty());
        let expected = golden("get_record_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_oai_datacite_response() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/0803"), Some("oai_datacite"));
        let xml = handle_get_record(&params, &repo_with_incunabula(), &InMemoryRecordRepository::empty());
        let expected = golden("get_record_oai_datacite.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- record golden tests ----

    #[test]
    fn record_golden_oai_dc_response() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            Some("oai_dc"),
        );
        let xml = handle_get_record(&params, &InMemoryProjectRepository::new(vec![]), &repo_with_record());
        let expected = golden("get_record_record_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn record_golden_oai_datacite_response() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            Some("oai_datacite"),
        );
        let xml = handle_get_record(&params, &InMemoryProjectRepository::new(vec![]), &repo_with_record());
        let expected = golden("get_record_record_oai_datacite.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn get_record_oai_dc_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/0803"), Some("oai_dc"));
        let xml = handle_get_record(&params, &repo_with_incunabula(), &InMemoryRecordRepository::empty());
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn get_record_oai_datacite_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/0803"), Some("oai_datacite"));
        let xml = handle_get_record(&params, &repo_with_incunabula(), &InMemoryRecordRepository::empty());
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn record_get_record_oai_dc_response_is_valid_oai_pmh() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            Some("oai_dc"),
        );
        let xml = handle_get_record(&params, &InMemoryProjectRepository::new(vec![]), &repo_with_record());
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn record_get_record_oai_datacite_response_is_valid_oai_pmh() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            Some("oai_datacite"),
        );
        let xml = handle_get_record(&params, &InMemoryProjectRepository::new(vec![]), &repo_with_record());
        crate::handlers::test_utils::validate_against_schema(&xml);
    }
}
