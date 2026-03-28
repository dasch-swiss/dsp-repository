//! OAI-PMH verb handlers.
//!
//! This module implements the six required OAI-PMH 2.0 verbs:
//! - Identify
//! - ListMetadataFormats
//! - ListSets
//! - ListIdentifiers
//! - ListRecords
//! - GetRecord

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

use dpe_core::{FsProjectRepository, FsRecordRepository, ProjectRepository, RecordRepository};

use super::error::OaiError;
use super::xml::OaiXmlBuilder;
use crate::metadata::{
    matches_date_filter, matches_date_filter_record, to_oai_record, to_oai_record_from_record, OaiRecord,
};

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
    let repo = FsProjectRepository::new();
    let record_repo = FsRecordRepository::new();

    let xml = match params.verb.as_deref() {
        Some("Identify") => handle_identify(&params, &repo),
        Some("ListMetadataFormats") => handle_list_metadata_formats(&params, &repo),
        Some("ListSets") => handle_list_sets(&params),
        Some("ListIdentifiers") => handle_list_identifiers(&params, &repo, &record_repo),
        Some("ListRecords") => handle_list_records(&params, &repo, &record_repo),
        Some("GetRecord") => handle_get_record(&params, &repo, &record_repo),
        Some(_) => build_error_response(OaiError::BadVerb, None),
        None => build_error_response(OaiError::BadVerb, None),
    };

    (StatusCode::OK, [(header::CONTENT_TYPE, "text/xml; charset=utf-8")], xml)
}

/// Builds an error response. Pass `Some(verb)` for recognized verbs so the verb is echoed
/// in the request element per OAI-PMH 2.0 section 3.6. Pass `None` only for badVerb.
pub fn build_error_response(error: OaiError, verb: Option<&str>) -> String {
    let mut builder = OaiXmlBuilder::new();
    match verb {
        Some(v) => builder.write_error_request_with_verb(v),
        None => builder.write_error_request(),
    }
    builder.write_error(&error);
    builder.finish()
}

/// Parses the set filter and returns (include_clusters, include_projects, include_records).
pub fn parse_set_filter(set: Option<&str>) -> (bool, bool, bool) {
    match set {
        Some("entityType:ProjectCluster") => (true, false, false),
        Some("entityType:ResearchProject") => (false, true, false),
        Some("entityType:Record") => (false, false, true),
        None => (true, true, true),
        Some(_) => (false, false, false), // Unknown set
    }
}

/// Validates the common parameters for ListIdentifiers and ListRecords and returns
/// the validated metadata prefix and the filtered list of OAI records.
///
/// Returns `Err(OaiError)` if any validation step fails.
pub fn validate_list_params<'a>(
    params: &'a OaiParams,
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
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
        return Err(OaiError::BadArgument("identifier argument is not allowed".to_string()));
    }

    // We don't support resumption tokens in v1
    if params.resumption_token.is_some() {
        return Err(OaiError::BadResumptionToken);
    }

    // Parse set filter
    let (include_clusters, include_projects, include_records) = parse_set_filter(params.set.as_deref());
    if !include_clusters && !include_projects && !include_records {
        return Err(OaiError::NoRecordsMatch);
    }

    let from = params.from.as_deref();
    let until = params.until.as_deref();

    // Collect project OAI records (clusters not yet implemented)
    let mut oai_records: Vec<OaiRecord> = if include_projects {
        repo.get_all()
            .iter()
            .filter(|p| matches_date_filter(p, from, until))
            .map(|p| to_oai_record(p, prefix))
            .collect()
    } else {
        Vec::new()
    };

    // Collect record OAI records
    if include_records {
        let mut record_oai: Vec<OaiRecord> = record_repo
            .get_all()
            .iter()
            .filter(|r| matches_date_filter_record(r, from, until))
            .map(|r| to_oai_record_from_record(r, prefix))
            .collect();
        oai_records.append(&mut record_oai);
    }

    if oai_records.is_empty() {
        return Err(OaiError::NoRecordsMatch);
    }

    Ok((prefix, oai_records))
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
pub mod test_utils;

#[cfg(test)]
mod tests {
    use super::{build_error_response, parse_set_filter};
    use crate::error::OaiError;

    #[test]
    fn test_parse_set_filter() {
        assert_eq!(parse_set_filter(None), (true, true, true));
        assert_eq!(parse_set_filter(Some("entityType:ProjectCluster")), (true, false, false));
        assert_eq!(parse_set_filter(Some("entityType:ResearchProject")), (false, true, false));
        assert_eq!(parse_set_filter(Some("entityType:Record")), (false, false, true));
        assert_eq!(parse_set_filter(Some("unknown")), (false, false, false));
    }

    #[test]
    fn bad_verb_error_omits_verb_attribute() {
        let xml = build_error_response(OaiError::BadVerb, None);
        assert!(xml.contains("<error code=\"badVerb\">"), "got: {}", xml);
        assert!(!xml.contains("verb="), "badVerb must not echo a verb attribute, got: {}", xml);
    }

    #[test]
    fn recognized_verb_error_echoes_verb_attribute() {
        let xml = build_error_response(OaiError::BadArgument("test".to_string()), Some("ListRecords"));
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(
            xml.contains("verb=\"ListRecords\""),
            "verb should be echoed in request element, got: {}",
            xml
        );
    }
}
