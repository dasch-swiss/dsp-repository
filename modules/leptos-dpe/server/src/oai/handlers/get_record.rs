//! Handler for the OAI-PMH GetRecord verb.

use super::data::get_project_by_shortcode;
use super::{build_error_response, OaiParams, SUPPORTED_PREFIXES};
use crate::oai::error::OaiError;
use crate::oai::metadata::{parse_oai_identifier, ProjectOaiExt};
use crate::oai::xml::OaiXmlBuilder;

/// Handles the GetRecord verb.
pub fn handle_get_record(params: &OaiParams) -> String {
    // Both identifier and metadataPrefix are required
    let identifier = match &params.identifier {
        Some(id) => id,
        None => {
            return build_error_response(OaiError::BadArgument(
                "identifier argument is required".to_string(),
            ))
        }
    };

    let prefix = match &params.metadata_prefix {
        Some(p) => p,
        None => {
            return build_error_response(OaiError::BadArgument(
                "metadataPrefix argument is required".to_string(),
            ))
        }
    };

    // Validate metadataPrefix
    if !SUPPORTED_PREFIXES.contains(&prefix.as_str()) {
        return build_error_response(OaiError::CannotDisseminateFormat);
    }

    // No other arguments allowed
    if params.from.is_some()
        || params.until.is_some()
        || params.set.is_some()
        || params.resumption_token.is_some()
    {
        return build_error_response(OaiError::BadArgument(
            "Unexpected argument for GetRecord".to_string(),
        ));
    }

    // Parse identifier and find project
    let shortcode = match parse_oai_identifier(identifier) {
        Some(sc) => sc,
        None => return build_error_response(OaiError::IdDoesNotExist),
    };

    let project = match get_project_by_shortcode(&shortcode) {
        Some(p) => p,
        None => return build_error_response(OaiError::IdDoesNotExist),
    };

    let record = project.to_oai_record(prefix);

    let mut builder = OaiXmlBuilder::new();
    builder.write_request(
        "GetRecord",
        &[
            ("identifier", identifier.as_str()),
            ("metadataPrefix", prefix.as_str()),
        ],
    );

    builder.start_element("GetRecord");
    builder.write_record(&record);
    builder.end_element("GetRecord");

    builder.finish()
}
