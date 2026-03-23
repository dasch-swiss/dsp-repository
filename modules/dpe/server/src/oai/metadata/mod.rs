//! Metadata transformation for OAI-PMH records.
//!
//! This module handles the transformation of Research Projects and Records into Dublin Core
//! and DataCite 4.6 metadata formats, following the DaSCH Metadata to DataCite
//! mapping specification.

mod datacite;
mod dublin_core;
mod helpers;
mod record_datacite;
mod record_dublin_core;
mod types;

pub use types::{DataCiteRecord, DublinCoreRecord, OaiRecord, OaiRecordHeader};

use datacite::project_to_datacite;
use dublin_core::project_to_dublin_core;
use record_datacite::record_to_datacite;
use record_dublin_core::record_to_dublin_core;

use app::domain::{record_datestamp, Project, Record};

const OAI_IDENTIFIER_PREFIX: &str = "oai:meta.dasch.swiss:";

/// Creates an OAI identifier from a project shortcode.
pub fn make_oai_identifier(shortcode: &str) -> String {
    format!("{}ark:/72163/1/{}", OAI_IDENTIFIER_PREFIX, shortcode)
}

/// Parses an OAI identifier and extracts the shortcode.
pub fn parse_oai_identifier(identifier: &str) -> Option<String> {
    if !identifier.starts_with(OAI_IDENTIFIER_PREFIX) {
        return None;
    }
    let ark_part = &identifier[OAI_IDENTIFIER_PREFIX.len()..];
    // Expected format: ark:/72163/1/{shortcode}
    ark_part.strip_prefix("ark:/72163/1/").map(|s| s.to_string())
}

/// Creates an OAI record from a project for the given metadata prefix.
pub fn to_oai_record(project: &Project, metadata_prefix: &str) -> OaiRecord {
    let header = OaiRecordHeader {
        identifier: make_oai_identifier(&project.shortcode),
        datestamp: if project.start_date != "MISSING" && !project.start_date.is_empty() {
            project.start_date.clone()
        } else {
            "2015-01-01".to_string()
        },
        set_specs: vec!["entityType:ResearchProject".to_string()],
    };

    let dublin_core = if metadata_prefix == "oai_dc" {
        Some(project_to_dublin_core(project))
    } else {
        None
    };

    let datacite = if metadata_prefix == "oai_datacite" {
        Some(project_to_datacite(project))
    } else {
        None
    };

    OaiRecord { header, dublin_core, datacite }
}

/// Extracts the last path segment from a full ARK URL.
///
/// `"https://ark.dasch.swiss/ark:/72163/1/record-0001"` → `"record-0001"`
pub fn ark_suffix_from_pid(pid: &str) -> Option<&str> {
    pid.rsplit('/').next().filter(|s| !s.is_empty())
}

/// Creates an OAI record from a Record for the given metadata prefix.
pub fn to_oai_record_from_record(record: &Record, metadata_prefix: &str) -> OaiRecord {
    let suffix = ark_suffix_from_pid(&record.pid).unwrap_or(&record.id);
    let header = OaiRecordHeader {
        identifier: make_oai_identifier(suffix),
        datestamp: record_datestamp(record),
        set_specs: vec!["entityType:Record".to_string()],
    };

    let dublin_core = if metadata_prefix == "oai_dc" {
        Some(record_to_dublin_core(record))
    } else {
        None
    };

    let datacite = if metadata_prefix == "oai_datacite" {
        Some(record_to_datacite(record))
    } else {
        None
    };

    OaiRecord { header, dublin_core, datacite }
}

/// Checks if a record matches the given date filter.
pub fn matches_date_filter_record(record: &Record, from: Option<&str>, until: Option<&str>) -> bool {
    let datestamp = record_datestamp(record);
    if let Some(from_date) = from {
        if datestamp.as_str() < from_date {
            return false;
        }
    }
    if let Some(until_date) = until {
        if datestamp.as_str() > until_date {
            return false;
        }
    }
    true
}

/// Checks if a project matches the given date filter.
pub fn matches_date_filter(project: &Project, from: Option<&str>, until: Option<&str>) -> bool {
    let datestamp = if project.start_date != "MISSING" && !project.start_date.is_empty() {
        &project.start_date
    } else {
        "2015-01-01"
    };

    if let Some(from_date) = from {
        if datestamp < from_date {
            return false;
        }
    }

    if let Some(until_date) = until {
        if datestamp > until_date {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::helpers::{
        extract_year, format_date_range, infer_subject_scheme, is_creator,
        license_identifier_to_label, map_contributor_type,
    };
    use super::{make_oai_identifier, parse_oai_identifier};

    #[test]
    fn test_make_oai_identifier() {
        let id = make_oai_identifier("0801");
        assert_eq!(id, "oai:meta.dasch.swiss:ark:/72163/1/0801");
    }

    #[test]
    fn test_parse_oai_identifier() {
        let shortcode = parse_oai_identifier("oai:meta.dasch.swiss:ark:/72163/1/0801");
        assert_eq!(shortcode, Some("0801".to_string()));
    }

    #[test]
    fn test_parse_oai_identifier_invalid() {
        let shortcode = parse_oai_identifier("invalid:identifier");
        assert_eq!(shortcode, None);
    }

    #[test]
    fn test_extract_year() {
        assert_eq!(extract_year("2024-01-15"), "2024");
        assert_eq!(extract_year("2024"), "2024");
        assert_eq!(extract_year("MISSING"), "2015");
    }

    #[test]
    fn test_is_creator_case_insensitive() {
        assert!(is_creator(&["Project Leader".to_string()]));
        assert!(is_creator(&["project leader".to_string()]));
        assert!(is_creator(&["Principal Investigator (PI)".to_string()]));
        assert!(is_creator(&["principal investigator (pi)".to_string()]));
        assert!(is_creator(&["Author".to_string()]));
        assert!(is_creator(&["author".to_string()]));
        assert!(!is_creator(&["Researcher".to_string()]));
        assert!(!is_creator(&["Data Collector".to_string()]));
    }

    #[test]
    fn test_is_creator_multiple_types() {
        assert!(is_creator(&[
            "Researcher".to_string(),
            "Project Leader".to_string()
        ]));
        assert!(!is_creator(&[
            "Researcher".to_string(),
            "Data Collector".to_string()
        ]));
    }

    #[test]
    fn test_format_date_range_both() {
        assert_eq!(
            format_date_range("2020-01-01", "2023-12-31"),
            Some("2020-01-01/2023-12-31".to_string())
        );
    }

    #[test]
    fn test_format_date_range_start_only() {
        assert_eq!(
            format_date_range("2020-01-01", "MISSING"),
            Some("2020-01-01".to_string())
        );
    }

    #[test]
    fn test_format_date_range_end_only() {
        assert_eq!(
            format_date_range("MISSING", "2023-12-31"),
            Some("2023-12-31".to_string())
        );
    }

    #[test]
    fn test_format_date_range_none() {
        assert_eq!(format_date_range("MISSING", "MISSING"), None);
    }

    #[test]
    fn test_map_contributor_type() {
        assert_eq!(map_contributor_type("Researcher"), "Researcher");
        assert_eq!(map_contributor_type("researcher"), "Researcher");
        assert_eq!(map_contributor_type("Data Collector"), "DataCollector");
        assert_eq!(map_contributor_type("data collector"), "DataCollector");
        assert_eq!(map_contributor_type("Unknown Role"), "Other");
    }

    #[test]
    fn test_license_identifier_to_label() {
        assert_eq!(
            license_identifier_to_label("CC-BY-4.0"),
            "Creative Commons Attribution 4.0 International"
        );
        assert_eq!(
            license_identifier_to_label("CC-BY-NC-SA-4.0"),
            "Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International"
        );
        assert_eq!(license_identifier_to_label("UNKNOWN"), "UNKNOWN");
    }

    #[test]
    fn test_infer_subject_scheme_gnd() {
        let (scheme, _uri) = infer_subject_scheme("https://d-nb.info/gnd/4066562-8");
        assert_eq!(scheme, Some("GND".to_string()));
    }

    #[test]
    fn test_infer_subject_scheme_unknown() {
        let (scheme, uri) = infer_subject_scheme("https://example.com/subject/123");
        assert_eq!(scheme, None);
        assert_eq!(uri, Some("https://example.com/subject/123".to_string()));
    }
}
