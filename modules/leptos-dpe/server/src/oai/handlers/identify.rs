//! Handler for the OAI-PMH Identify verb.

use super::data::get_earliest_datestamp;
use super::{build_error_response, OaiParams};
use crate::oai::error::OaiError;
use crate::oai::xml::OaiXmlBuilder;

/// Handles the Identify verb.
pub fn handle_identify(params: &OaiParams) -> String {
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

    // Calculate earliest datestamp from projects
    let earliest = get_earliest_datestamp();
    builder.write_identify(&earliest);

    builder.finish()
}
