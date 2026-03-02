//! OAI-PMH verb handlers.
//!
//! This module implements the six required OAI-PMH 2.0 verbs:
//! - Identify
//! - ListMetadataFormats
//! - ListSets
//! - ListIdentifiers
//! - ListRecords
//! - GetRecord

use axum::{
    extract::Query,
    http::{header, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use std::fs;
use std::path::Path;

use super::error::OaiError;
use super::metadata::{parse_oai_identifier, OaiRecord, ProjectOaiExt};
use super::xml::{OaiXmlBuilder, EARLIEST_DATESTAMP};
use app::domain::Project;

/// Query parameters for OAI-PMH requests.
#[derive(Debug, Deserialize)]
pub struct OaiParams {
    verb: Option<String>,
    identifier: Option<String>,
    #[serde(rename = "metadataPrefix")]
    metadata_prefix: Option<String>,
    from: Option<String>,
    until: Option<String>,
    set: Option<String>,
    #[serde(rename = "resumptionToken")]
    resumption_token: Option<String>,
}

/// Supported metadata formats.
const SUPPORTED_PREFIXES: [&str; 2] = ["oai_dc", "oai_datacite"];

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
fn build_error_response(error: OaiError) -> String {
    let mut builder = OaiXmlBuilder::new();
    builder.write_error_request();
    builder.write_error(&error);
    builder.finish()
}

/// Handles the Identify verb.
fn handle_identify(params: &OaiParams) -> String {
    // Identify does not accept any parameters except verb
    if params.identifier.is_some()
        || params.metadata_prefix.is_some()
        || params.from.is_some()
        || params.until.is_some()
        || params.set.is_some()
        || params.resumption_token.is_some()
    {
        return build_error_response(OaiError::BadArgument("Identify does not accept any arguments".to_string()));
    }

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("Identify", &[]);

    // Calculate earliest datestamp from projects
    let earliest = get_earliest_datestamp();
    builder.write_identify(&earliest);

    builder.finish()
}

/// Handles the ListMetadataFormats verb.
fn handle_list_metadata_formats(params: &OaiParams) -> String {
    // ListMetadataFormats accepts only identifier as optional argument
    if params.from.is_some() || params.until.is_some() || params.set.is_some() || params.resumption_token.is_some() {
        return build_error_response(OaiError::BadArgument("Unexpected argument for ListMetadataFormats".to_string()));
    }

    // If identifier is provided, verify it exists
    if let Some(ref id) = params.identifier {
        if let Some(shortcode) = parse_oai_identifier(id) {
            if get_project_by_shortcode(&shortcode).is_none() {
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

/// Handles the ListSets verb.
fn handle_list_sets(params: &OaiParams) -> String {
    // ListSets accepts only resumptionToken
    if params.identifier.is_some()
        || params.metadata_prefix.is_some()
        || params.from.is_some()
        || params.until.is_some()
    {
        return build_error_response(OaiError::BadArgument("Unexpected argument for ListSets".to_string()));
    }

    // We don't support resumption tokens in v1
    if params.resumption_token.is_some() {
        return build_error_response(OaiError::BadArgument("Resumption tokens are not supported".to_string()));
    }

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("ListSets", &[]);
    builder.write_list_sets();

    builder.finish()
}

/// Handles the ListIdentifiers verb.
fn handle_list_identifiers(params: &OaiParams) -> String {
    // metadataPrefix is required
    let prefix = match &params.metadata_prefix {
        Some(p) => p,
        None => return build_error_response(OaiError::BadArgument("metadataPrefix argument is required".to_string())),
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
        return build_error_response(OaiError::BadArgument("Resumption tokens are not supported".to_string()));
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
        .filter(|p| include_projects && p.matches_date_filter(params.from.as_deref(), params.until.as_deref()))
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
        builder.write_record_header(&record.header.identifier, &record.header.datestamp, &record.header.set_specs);
    }
    builder.end_element("ListIdentifiers");

    builder.finish()
}

/// Handles the ListRecords verb.
fn handle_list_records(params: &OaiParams) -> String {
    // metadataPrefix is required
    let prefix = match &params.metadata_prefix {
        Some(p) => p,
        None => return build_error_response(OaiError::BadArgument("metadataPrefix argument is required".to_string())),
    };

    // Validate metadataPrefix
    if !SUPPORTED_PREFIXES.contains(&prefix.as_str()) {
        return build_error_response(OaiError::CannotDisseminateFormat);
    }

    // identifier is not valid for ListRecords
    if params.identifier.is_some() {
        return build_error_response(OaiError::BadArgument(
            "identifier argument is not allowed for ListRecords".to_string(),
        ));
    }

    // We don't support resumption tokens in v1
    if params.resumption_token.is_some() {
        return build_error_response(OaiError::BadArgument("Resumption tokens are not supported".to_string()));
    }

    // Parse set filter
    let (include_clusters, include_projects) = parse_set_filter(params.set.as_deref());
    if !include_clusters && !include_projects {
        return build_error_response(OaiError::BadArgument("Unknown set".to_string()));
    }

    // Get projects and filter
    let projects = get_all_projects();
    let filtered: Vec<OaiRecord> = projects
        .iter()
        .filter(|p| include_projects && p.matches_date_filter(params.from.as_deref(), params.until.as_deref()))
        .map(|p| p.to_oai_record(prefix))
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
    builder.write_request("ListRecords", &request_params);

    builder.start_element("ListRecords");
    for record in &filtered {
        builder.write_record(record);
    }
    builder.end_element("ListRecords");

    builder.finish()
}

/// Handles the GetRecord verb.
fn handle_get_record(params: &OaiParams) -> String {
    // Both identifier and metadataPrefix are required
    let identifier = match &params.identifier {
        Some(id) => id,
        None => return build_error_response(OaiError::BadArgument("identifier argument is required".to_string())),
    };

    let prefix = match &params.metadata_prefix {
        Some(p) => p,
        None => return build_error_response(OaiError::BadArgument("metadataPrefix argument is required".to_string())),
    };

    // Validate metadataPrefix
    if !SUPPORTED_PREFIXES.contains(&prefix.as_str()) {
        return build_error_response(OaiError::CannotDisseminateFormat);
    }

    // No other arguments allowed
    if params.from.is_some() || params.until.is_some() || params.set.is_some() || params.resumption_token.is_some() {
        return build_error_response(OaiError::BadArgument("Unexpected argument for GetRecord".to_string()));
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
        &[("identifier", identifier.as_str()), ("metadataPrefix", prefix.as_str())],
    );

    builder.start_element("GetRecord");
    builder.write_record(&record);
    builder.end_element("GetRecord");

    builder.finish()
}

/// Parses the set filter and returns (include_clusters, include_projects).
fn parse_set_filter(set: Option<&str>) -> (bool, bool) {
    match set {
        Some("entityType:ProjectCluster") => (true, false),
        Some("entityType:ResearchProject") => (false, true),
        None => (true, true),
        Some(_) => (false, false), // Unknown set
    }
}

/// Gets the data directory path.
fn get_data_dir() -> String {
    if let Ok(data_dir) = std::env::var("DATA_DIR") {
        return data_dir;
    }
    "modules/leptos-dpe/server/data".to_string()
}

/// Loads all projects from the data directory.
fn get_all_projects() -> Vec<Project> {
    let projects_dir = format!("{}/projects", get_data_dir());
    let path = Path::new(&projects_dir);

    if !path.exists() {
        return Vec::new();
    }

    let mut projects = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    if let Ok(project) = serde_json::from_str::<Project>(&content) {
                        projects.push(project);
                    }
                }
            }
        }
    }

    projects
}

/// Gets a project by its shortcode.
fn get_project_by_shortcode(shortcode: &str) -> Option<Project> {
    let projects_dir = format!("{}/projects", get_data_dir());
    let path = Path::new(&projects_dir);

    if !path.exists() {
        return None;
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    if let Ok(project) = serde_json::from_str::<Project>(&content) {
                        if project.shortcode == shortcode {
                            return Some(project);
                        }
                    }
                }
            }
        }
    }

    None
}

/// Gets the earliest datestamp from all projects.
fn get_earliest_datestamp() -> String {
    let projects = get_all_projects();

    projects
        .iter()
        .filter_map(|p| {
            if p.start_date != "MISSING" && !p.start_date.is_empty() {
                Some(p.start_date.as_str())
            } else {
                None
            }
        })
        .min()
        .unwrap_or(EARLIEST_DATESTAMP)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_set_filter() {
        assert_eq!(parse_set_filter(None), (true, true));
        assert_eq!(parse_set_filter(Some("entityType:ProjectCluster")), (true, false));
        assert_eq!(parse_set_filter(Some("entityType:ResearchProject")), (false, true));
        assert_eq!(parse_set_filter(Some("unknown")), (false, false));
    }
}
