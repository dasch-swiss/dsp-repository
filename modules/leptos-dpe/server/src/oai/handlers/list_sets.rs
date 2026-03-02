//! Handler for the OAI-PMH ListSets verb.

use super::{build_error_response, OaiParams};
use crate::oai::error::OaiError;
use crate::oai::xml::OaiXmlBuilder;

/// Handles the ListSets verb.
pub fn handle_list_sets(params: &OaiParams) -> String {
    // ListSets accepts only resumptionToken
    if params.identifier.is_some()
        || params.metadata_prefix.is_some()
        || params.from.is_some()
        || params.until.is_some()
    {
        return build_error_response(OaiError::BadArgument(
            "Unexpected argument for ListSets".to_string(),
        ));
    }

    // We don't support resumption tokens in v1
    if params.resumption_token.is_some() {
        return build_error_response(OaiError::BadArgument(
            "Resumption tokens are not supported".to_string(),
        ));
    }

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("ListSets", &[]);
    builder.write_list_sets();

    builder.finish()
}
