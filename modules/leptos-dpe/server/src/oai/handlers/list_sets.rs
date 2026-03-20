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
        return build_error_response(OaiError::BadResumptionToken);
    }

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("ListSets", &[]);
    builder.write_list_sets();

    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_params() -> OaiParams {
        OaiParams {
            verb: Some("ListSets".to_string()),
            identifier: None,
            metadata_prefix: None,
            from: None,
            until: None,
            set: None,
            resumption_token: None,
        }
    }

    use super::super::test_utils::{golden, normalize};

    // ---- error cases ----

    #[test]
    fn unexpected_argument_returns_bad_argument() {
        let mut params = make_params();
        params.from = Some("2020-01-01".to_string());
        let xml = handle_list_sets(&params);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("Unexpected argument for ListSets"), "got: {}", xml);
    }

    #[test]
    fn resumption_token_returns_bad_resumption_token() {
        let mut params = make_params();
        params.resumption_token = Some("some-token".to_string());
        let xml = handle_list_sets(&params);
        assert!(xml.contains("<error code=\"badResumptionToken\">"), "got: {}", xml);
    }

    // ---- golden tests ----

    #[test]
    fn golden_list_sets_response() {
        let params = make_params();
        let xml = handle_list_sets(&params);
        let expected = golden("list_sets.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn list_sets_response_is_valid_oai_pmh() {
        let params = make_params();
        let xml = handle_list_sets(&params);
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }
}
