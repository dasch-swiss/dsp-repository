//! Handler for the OAI-PMH ListRecords verb.

use app::domain::ProjectRepository;

use super::{build_error_response, build_list_request_params, validate_list_params, OaiParams};
use crate::oai::xml::OaiXmlBuilder;

/// Handles the ListRecords verb.
pub fn handle_list_records(params: &OaiParams, repo: &dyn ProjectRepository) -> String {
    let (prefix, records) = match validate_list_params(params, repo) {
        Ok(result) => result,
        Err(err) => return build_error_response(err),
    };

    let request_params = build_list_request_params(prefix, params);

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("ListRecords", &request_params);

    builder.start_element("ListRecords");
    for record in &records {
        builder.write_record(record);
    }
    builder.end_element("ListRecords");

    builder.finish()
}
