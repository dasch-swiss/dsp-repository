//! Handler for the OAI-PMH ListMetadataFormats verb.

use dpe_core::ProjectRepository;

use super::{build_error_response, OaiParams};
use crate::error::OaiError;
use crate::metadata::parse_oai_identifier;
use crate::xml::OaiXmlBuilder;

/// Handles the ListMetadataFormats verb.
pub fn handle_list_metadata_formats(params: &OaiParams, repo: &dyn ProjectRepository) -> String {
    // ListMetadataFormats accepts only identifier as optional argument
    if params.from.is_some()
        || params.until.is_some()
        || params.set.is_some()
        || params.resumption_token.is_some()
    {
        return build_error_response(
            OaiError::BadArgument("Unexpected argument for ListMetadataFormats".to_string()),
            Some("ListMetadataFormats"),
        );
    }

    // If identifier is provided, verify it exists
    if let Some(ref id) = params.identifier {
        if let Some(shortcode) = parse_oai_identifier(id) {
            if repo.get_by_shortcode(&shortcode).is_none() {
                return build_error_response(OaiError::IdDoesNotExist, Some("ListMetadataFormats"));
            }
        } else {
            return build_error_response(OaiError::IdDoesNotExist, Some("ListMetadataFormats"));
        }
    }

    let mut builder = OaiXmlBuilder::new();
    let mut request_params = vec![];
    if let Some(ref id) = params.identifier {
        request_params.push(("identifier", id.as_str()));
    }
    builder.write_request("ListMetadataFormats", &request_params);
    builder.write_list_metadata_formats();

    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::test_utils::{golden, incunabula_project, normalize, InMemoryProjectRepository};

    fn make_params(identifier: Option<&str>) -> OaiParams {
        OaiParams {
            verb: Some("ListMetadataFormats".to_string()),
            identifier: identifier.map(str::to_string),
            metadata_prefix: None,
            from: None,
            until: None,
            set: None,
            resumption_token: None,
        }
    }

    // ---- error cases ----

    #[test]
    fn unexpected_argument_returns_bad_argument() {
        let mut params = make_params(None);
        params.from = Some("2020-01-01".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_metadata_formats(&params, &repo);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_identifier_returns_id_does_not_exist() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/9999"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_metadata_formats(&params, &repo);
        assert!(xml.contains("<error code=\"idDoesNotExist\">"), "got: {}", xml);
    }

    // ---- golden tests ----

    #[test]
    fn golden_response() {
        let params = make_params(None);
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_metadata_formats(&params, &repo);
        let expected = golden("list_metadata_formats.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn list_metadata_formats_response_is_valid_oai_pmh() {
        let params = make_params(None);
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_metadata_formats(&params, &repo);
        crate::handlers::test_utils::validate_against_schema(&xml);
    }
}
