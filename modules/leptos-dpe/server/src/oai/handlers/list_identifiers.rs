//! Handler for the OAI-PMH ListIdentifiers verb.

use app::domain::ProjectRepository;

use super::{build_error_response, build_list_request_params, validate_list_params, OaiParams};
use crate::oai::xml::OaiXmlBuilder;

/// Handles the ListIdentifiers verb.
pub fn handle_list_identifiers(params: &OaiParams, repo: &dyn ProjectRepository) -> String {
    let (prefix, records) = match validate_list_params(params, repo) {
        Ok(result) => result,
        Err(err) => return build_error_response(err),
    };

    let request_params = build_list_request_params(prefix, params);

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("ListIdentifiers", &request_params);

    builder.start_element("ListIdentifiers");
    for record in &records {
        builder.write_record_header(
            &record.header.identifier,
            &record.header.datestamp,
            &record.header.set_specs,
        );
    }
    builder.end_element("ListIdentifiers");

    builder.finish()
}
