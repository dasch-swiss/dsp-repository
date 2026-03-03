//! OAI-PMH verb handlers.
//!
//! This module implements the six required OAI-PMH 2.0 verbs:
//! - Identify
//! - ListMetadataFormats
//! - ListSets
//! - ListIdentifiers
//! - ListRecords
//! - GetRecord

mod data;
mod get_record;
mod identify;
mod list_identifiers;
mod list_metadata_formats;
mod list_records;
mod list_sets;

use axum::{
    extract::Query,
    http::{header, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;

use super::error::OaiError;
use super::xml::OaiXmlBuilder;
use crate::oai::metadata::{OaiRecord, ProjectOaiExt};

use data::get_all_projects;
use get_record::handle_get_record;
use identify::handle_identify;
use list_identifiers::handle_list_identifiers;
use list_metadata_formats::handle_list_metadata_formats;
use list_records::handle_list_records;
use list_sets::handle_list_sets;

/// Query parameters for OAI-PMH requests.
#[derive(Debug, Deserialize)]
pub struct OaiParams {
    pub verb: Option<String>,
    pub identifier: Option<String>,
    #[serde(rename = "metadataPrefix")]
    pub metadata_prefix: Option<String>,
    pub from: Option<String>,
    pub until: Option<String>,
    pub set: Option<String>,
    #[serde(rename = "resumptionToken")]
    pub resumption_token: Option<String>,
}

/// Supported metadata formats.
pub const SUPPORTED_PREFIXES: [&str; 2] = ["oai_dc", "oai_datacite"];

/// Main OAI-PMH handler that dispatches to verb-specific handlers.
pub async fn oai_handler(Query(params): Query<OaiParams>) -> impl IntoResponse {
    let xml = match params.verb.as_deref() {
        Some("Identify") => handle_identify(&params),
        Some("ListMetadataFormats") => handle_list_metadata_formats(&params),
        Some("ListSets") => handle_list_sets(&params),
        Some("ListIdentifiers") => handle_list_identifiers(&params),
        Some("ListRecords") => handle_list_records(&params),
        Some("GetRecord") => handle_get_record(&params),
        Some(_) => build_error_response(OaiError::BadVerb),
        None => build_error_response(OaiError::BadVerb),
    };

    (StatusCode::OK, [(header::CONTENT_TYPE, "text/xml; charset=utf-8")], xml)
}

/// Builds an error response.
pub fn build_error_response(error: OaiError) -> String {
    let mut builder = OaiXmlBuilder::new();
    builder.write_error_request();
    builder.write_error(&error);
    builder.finish()
}

/// Parses the set filter and returns (include_clusters, include_projects).
pub fn parse_set_filter(set: Option<&str>) -> (bool, bool) {
    match set {
        Some("entityType:ProjectCluster") => (true, false),
        Some("entityType:ResearchProject") => (false, true),
        None => (true, true),
        Some(_) => (false, false), // Unknown set
    }
}

/// Validates the common parameters for ListIdentifiers and ListRecords and returns
/// the validated metadata prefix and the filtered list of OAI records.
///
/// Returns `Err(OaiError)` if any validation step fails.
pub fn validate_list_params<'a>(
    params: &'a OaiParams,
) -> Result<(&'a str, Vec<OaiRecord>), OaiError> {
    // metadataPrefix is required
    let prefix = params
        .metadata_prefix
        .as_deref()
        .ok_or_else(|| OaiError::BadArgument("metadataPrefix argument is required".to_string()))?;

    // Validate metadataPrefix
    if !SUPPORTED_PREFIXES.contains(&prefix) {
        return Err(OaiError::CannotDisseminateFormat);
    }

    // identifier is not valid for list verbs
    if params.identifier.is_some() {
        return Err(OaiError::BadArgument(
            "identifier argument is not allowed".to_string(),
        ));
    }

    // We don't support resumption tokens in v1
    if params.resumption_token.is_some() {
        return Err(OaiError::BadResumptionToken);
    }

    // Parse set filter
    let (include_clusters, include_projects) = parse_set_filter(params.set.as_deref());
    if !include_clusters && !include_projects {
        return Err(OaiError::NoRecordsMatch);
    }

    // Get projects and filter
    let projects = get_all_projects();
    let filtered: Vec<OaiRecord> = projects
        .iter()
        .filter(|p| {
            include_projects
                && p.matches_date_filter(params.from.as_deref(), params.until.as_deref())
        })
        .map(|p| p.to_oai_record(prefix))
        .collect();

    // Currently we only have projects, no clusters
    if filtered.is_empty() {
        return Err(OaiError::NoRecordsMatch);
    }

    Ok((prefix, filtered))
}

/// Builds the request parameter list shared by list verb XML responses.
pub fn build_list_request_params<'a>(prefix: &'a str, params: &'a OaiParams) -> Vec<(&'a str, &'a str)> {
    let mut request_params = vec![("metadataPrefix", prefix)];
    if let Some(ref from) = params.from {
        request_params.push(("from", from.as_str()));
    }
    if let Some(ref until) = params.until {
        request_params.push(("until", until.as_str()));
    }
    if let Some(ref set) = params.set {
        request_params.push(("set", set.as_str()));
    }
    request_params
}

#[cfg(test)]
mod tests {
    use super::parse_set_filter;

    #[test]
    fn test_parse_set_filter() {
        assert_eq!(parse_set_filter(None), (true, true));
        assert_eq!(parse_set_filter(Some("entityType:ProjectCluster")), (true, false));
        assert_eq!(parse_set_filter(Some("entityType:ResearchProject")), (false, true));
        assert_eq!(parse_set_filter(Some("unknown")), (false, false));
    }
}
