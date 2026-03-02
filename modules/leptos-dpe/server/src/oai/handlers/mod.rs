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

#[cfg(test)]
mod tests {
    use super::data::parse_set_filter;

    #[test]
    fn test_parse_set_filter() {
        assert_eq!(parse_set_filter(None), (true, true));
        assert_eq!(parse_set_filter(Some("entityType:ProjectCluster")), (true, false));
        assert_eq!(parse_set_filter(Some("entityType:ResearchProject")), (false, true));
        assert_eq!(parse_set_filter(Some("unknown")), (false, false));
    }
}
