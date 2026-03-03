//! Handler for the OAI-PMH ListMetadataFormats verb.

use app::domain::ProjectRepository;

use super::{build_error_response, OaiParams};
use crate::oai::error::OaiError;
use crate::oai::metadata::parse_oai_identifier;
use crate::oai::xml::OaiXmlBuilder;

/// Handles the ListMetadataFormats verb.
pub fn handle_list_metadata_formats(params: &OaiParams, repo: &dyn ProjectRepository) -> String {
    // ListMetadataFormats accepts only identifier as optional argument
    if params.from.is_some()
        || params.until.is_some()
        || params.set.is_some()
        || params.resumption_token.is_some()
    {
        return build_error_response(OaiError::BadArgument(
            "Unexpected argument for ListMetadataFormats".to_string(),
        ));
    }

    // If identifier is provided, verify it exists
    if let Some(ref id) = params.identifier {
        if let Some(shortcode) = parse_oai_identifier(id) {
            if repo.get_by_shortcode(&shortcode).is_none() {
                return build_error_response(OaiError::IdDoesNotExist);
            }
        } else {
            return build_error_response(OaiError::IdDoesNotExist);
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
