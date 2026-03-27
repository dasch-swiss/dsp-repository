//! Handler for the OAI-PMH Identify verb.

use dpe_app::domain::ProjectRepository;

use super::{build_error_response, OaiParams};
use crate::oai::error::OaiError;
use crate::oai::xml::{OaiXmlBuilder, EARLIEST_DATESTAMP};

/// Handles the Identify verb.
pub fn handle_identify(params: &OaiParams, repo: &dyn ProjectRepository) -> String {
    // Identify does not accept any parameters except verb
    if params.identifier.is_some()
        || params.metadata_prefix.is_some()
        || params.from.is_some()
        || params.until.is_some()
        || params.set.is_some()
        || params.resumption_token.is_some()
    {
        return build_error_response(
            OaiError::BadArgument("Identify does not accept any arguments".to_string()),
            Some("Identify"),
        );
    }

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("Identify", &[]);

    let earliest = get_earliest_datestamp(repo);
    builder.write_identify(&earliest);

    builder.finish()
}

fn get_earliest_datestamp(repo: &dyn ProjectRepository) -> String {
    let projects = repo.get_all();
    projects
        .iter()
        .filter_map(|p| {
            if p.start_date != "MISSING" && !p.start_date.is_empty() {
                Some(p.start_date.as_str())
            } else {
                None
            }
        })
        .min()
        .unwrap_or(EARLIEST_DATESTAMP)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::test_utils::{golden, incunabula_project, normalize, InMemoryProjectRepository};

    fn make_params() -> OaiParams {
        OaiParams {
            verb: Some("Identify".to_string()),
            identifier: None,
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
        let mut params = make_params();
        params.set = Some("entityType:ResearchProject".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_identify(&params, &repo);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("Identify does not accept any arguments"), "got: {}", xml);
        assert!(xml.contains("verb=\"Identify\""), "verb should be echoed in request element, got: {}", xml);
    }

    // ---- golden tests ----

    #[test]
    fn golden_identify_response() {
        let params = make_params();
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_identify(&params, &repo);
        let expected = golden("identify.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_identify_empty_repo_response() {
        let params = make_params();
        let repo = InMemoryProjectRepository::new(vec![]);
        let xml = handle_identify(&params, &repo);
        let expected = golden("identify_empty_repo.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn identify_response_is_valid_oai_pmh() {
        let params = make_params();
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_identify(&params, &repo);
        crate::oai::handlers::test_utils::validate_against_schema(&xml);

    }

    #[test]
    fn identify_empty_repo_response_is_valid_oai_pmh() {
        let params = make_params();
        let repo = InMemoryProjectRepository::new(vec![]);
        let xml = handle_identify(&params, &repo);
        crate::oai::handlers::test_utils::validate_against_schema(&xml);

    }
}
