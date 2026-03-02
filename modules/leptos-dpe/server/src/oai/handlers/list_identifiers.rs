//! Handler for the OAI-PMH ListIdentifiers verb.

use super::data::{get_all_projects, parse_set_filter};
use super::{build_error_response, OaiParams, SUPPORTED_PREFIXES};
use crate::oai::error::OaiError;
use crate::oai::metadata::ProjectOaiExt;
use crate::oai::xml::OaiXmlBuilder;

/// Handles the ListIdentifiers verb.
pub fn handle_list_identifiers(params: &OaiParams) -> String {
    // metadataPrefix is required
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

    // identifier is not valid for ListIdentifiers
    if params.identifier.is_some() {
        return build_error_response(OaiError::BadArgument(
            "identifier argument is not allowed for ListIdentifiers".to_string(),
        ));
    }

    // We don't support resumption tokens in v1
    if params.resumption_token.is_some() {
        return build_error_response(OaiError::BadArgument(
            "Resumption tokens are not supported".to_string(),
        ));
    }

    // Parse set filter
    let (include_clusters, include_projects) = parse_set_filter(params.set.as_deref());
    if !include_clusters && !include_projects {
        return build_error_response(OaiError::BadArgument("Unknown set".to_string()));
    }

    // Get projects and filter
    let projects = get_all_projects();
    let filtered: Vec<_> = projects
        .iter()
        .filter(|p| {
            include_projects && p.matches_date_filter(params.from.as_deref(), params.until.as_deref())
        })
        .collect();

    // Currently we only have projects, no clusters
    if filtered.is_empty() {
        return build_error_response(OaiError::NoRecordsMatch);
    }

    let mut builder = OaiXmlBuilder::new();
    let mut request_params = vec![("metadataPrefix", prefix.as_str())];
    if let Some(ref from) = params.from {
        request_params.push(("from", from.as_str()));
    }
    if let Some(ref until) = params.until {
        request_params.push(("until", until.as_str()));
    }
    if let Some(ref set) = params.set {
        request_params.push(("set", set.as_str()));
    }
    builder.write_request("ListIdentifiers", &request_params);

    builder.start_element("ListIdentifiers");
    for project in filtered {
        let record = project.to_oai_record(prefix);
        builder.write_record_header(
            &record.header.identifier,
            &record.header.datestamp,
            &record.header.set_specs,
        );
    }
    builder.end_element("ListIdentifiers");

    builder.finish()
}
