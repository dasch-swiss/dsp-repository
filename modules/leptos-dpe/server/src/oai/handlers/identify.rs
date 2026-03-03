//! Handler for the OAI-PMH Identify verb.

use app::domain::ProjectRepository;

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
        return build_error_response(OaiError::BadArgument(
            "Identify does not accept any arguments".to_string(),
        ));
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
