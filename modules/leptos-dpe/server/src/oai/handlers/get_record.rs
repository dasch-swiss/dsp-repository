//! Handler for the OAI-PMH GetRecord verb.

use app::domain::{Project, ProjectRepository};

use super::{build_error_response, OaiParams, SUPPORTED_PREFIXES};
use crate::oai::error::OaiError;
use crate::oai::metadata::{parse_oai_identifier, to_oai_record};
use crate::oai::xml::OaiXmlBuilder;

/// Handles the GetRecord verb.
pub fn handle_get_record(params: &OaiParams, repo: &dyn ProjectRepository) -> String {
    let result = require_identifier(params)
        .and_then(|id| require_metadata_prefix(params).map(|prefix| (id, prefix)))
        .and_then(|(id, prefix)| reject_unexpected_args(params).map(|_| (id, prefix)))
        .and_then(|(id, prefix)| resolve_project(id, repo).map(|project| (id, prefix, project)))
        .map(|(id, prefix, project)| build_response(id, prefix, &project));

    result.unwrap_or_else(build_error_response)
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
    if params.from.is_some()
        || params.until.is_some()
        || params.set.is_some()
        || params.resumption_token.is_some()
    {
        return Err(OaiError::BadArgument(
            "Unexpected argument for GetRecord".to_string(),
        ));
    }
    Ok(())
}

fn resolve_project(identifier: &str, repo: &dyn ProjectRepository) -> Result<Project, OaiError> {
    let shortcode = parse_oai_identifier(identifier).ok_or(OaiError::IdDoesNotExist)?;
    repo.get_by_shortcode(&shortcode).ok_or(OaiError::IdDoesNotExist)
}

fn build_response(identifier: &str, prefix: &str, project: &Project) -> String {
    let record = to_oai_record(project, prefix);

    let mut builder = OaiXmlBuilder::new();
    builder.write_request(
        "GetRecord",
        &[("identifier", identifier), ("metadataPrefix", prefix)],
    );
    builder.start_element("GetRecord");
    builder.write_record(&record);
    builder.end_element("GetRecord");
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::test_utils::{golden, incunabula_project, normalize, InMemoryProjectRepository};

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

    // ---- error cases ----

    #[test]
    fn missing_identifier_returns_bad_argument() {
        let params = make_params(None, Some("oai_dc"));
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("identifier argument is required"), "got: {}", xml);
    }

    #[test]
    fn missing_metadata_prefix_returns_bad_argument() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/0803"), None);
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("metadataPrefix argument is required"), "got: {}", xml);
    }

    #[test]
    fn unsupported_metadata_prefix_returns_cannot_disseminate() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/0803"), Some("marc21"));
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        assert!(xml.contains("<error code=\"cannotDisseminateFormat\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_shortcode_returns_id_does_not_exist() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/9999"),
            Some("oai_dc"),
        );
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        assert!(xml.contains("<error code=\"idDoesNotExist\">"), "got: {}", xml);
    }

    #[test]
    fn unexpected_argument_returns_bad_argument() {
        let mut params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803"),
            Some("oai_dc"),
        );
        params.set = Some("entityType:ResearchProject".to_string());
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    // ---- golden tests ----

    #[test]
    fn golden_oai_dc_response() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803"),
            Some("oai_dc"),
        );
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        let expected = golden("get_record_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_oai_datacite_response() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803"),
            Some("oai_datacite"),
        );
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        let expected = golden("get_record_oai_datacite.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn get_record_oai_dc_response_is_valid_oai_pmh() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803"),
            Some("oai_dc"),
        );
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn get_record_oai_datacite_response_is_valid_oai_pmh() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803"),
            Some("oai_datacite"),
        );
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }
}
