//! Shared helper functions for metadata transformation.

use shared_metadata::AccessRightsType;

/// Extracts the year from a date string (YYYY-MM-DD or YYYY).
pub fn extract_year(date: &str) -> String {
    if date.len() >= 4 && !shared_metadata::is_placeholder(date) {
        date[..4].to_string()
    } else {
        "2015".to_string() // Default fallback year
    }
}

/// Converts an AccessRightsType to a human-readable string.
pub fn access_rights_to_string(ar: &AccessRightsType) -> &'static str {
    match ar {
        AccessRightsType::FullOpenAccess => "Full Open Access",
        AccessRightsType::OpenAccessWithRestrictions => "Open Access with Restrictions",
        AccessRightsType::EmbargoedAccess => "Embargoed Access",
        AccessRightsType::MetadataOnlyAccess => "Metadata only Access",
    }
}

/// Checks whether an attribution represents a creator (principal investigator,
/// project leader, author, or creator) using case-insensitive matching.
pub fn is_creator(contributor_types: &[String]) -> bool {
    contributor_types.iter().any(|t| {
        let lower = t.to_lowercase();
        lower == "project leader" || lower == "principal investigator (pi)" || lower == "author" || lower == "creator"
    })
}

/// Maps a contributor type string to the closest DataCite contributorType
/// vocabulary term.
pub fn map_contributor_type(contributor_type: &str) -> &'static str {
    match contributor_type.to_lowercase().as_str() {
        "researcher" => "Researcher",
        "data collector" => "DataCollector",
        "data curator" => "DataCurator",
        "data manager" => "DataManager",
        "editor" => "Editor",
        "producer" => "Producer",
        "supervisor" => "Supervisor",
        "sponsor" => "Sponsor",
        "research group" => "ResearchGroup",
        "distributor" => "Distributor",
        "hosting institution" => "HostingInstitution",
        "rights holder" => "RightsHolder",
        _ => "Other",
    }
}

/// Formats a date range from startDate and endDate.
/// Returns "startDate/endDate" when both are valid, or just the valid one.
pub fn format_date_range(start: &str, end: &str) -> Option<String> {
    let has_start = !shared_metadata::is_placeholder(start) && !start.is_empty();
    let has_end = !shared_metadata::is_placeholder(end) && !end.is_empty();
    match (has_start, has_end) {
        (true, true) => Some(format!("{}/{}", start, end)),
        (true, false) => Some(start.to_string()),
        (false, true) => Some(end.to_string()),
        (false, false) => None,
    }
}

/// Converts an SPDX license identifier to a human-readable label.
pub fn license_identifier_to_label(identifier: &str) -> String {
    match identifier {
        "CC-BY-4.0" => "Creative Commons Attribution 4.0 International".to_string(),
        "CC-BY-SA-4.0" => "Creative Commons Attribution-ShareAlike 4.0 International".to_string(),
        "CC-BY-NC-4.0" => "Creative Commons Attribution-NonCommercial 4.0 International".to_string(),
        "CC-BY-NC-SA-4.0" => "Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International".to_string(),
        "CC-BY-ND-4.0" => "Creative Commons Attribution-NoDerivatives 4.0 International".to_string(),
        "CC-BY-NC-ND-4.0" => "Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International".to_string(),
        "CC0-1.0" => "Creative Commons Public Domain Dedication".to_string(),
        _ => identifier.to_string(),
    }
}

/// Infers a subject scheme from an AuthorityFileReference URL.
pub fn infer_subject_scheme(url: &str) -> (Option<String>, Option<String>) {
    if url.contains("skos.um.es") || url.contains("zbw.eu/stw") {
        (Some("STW Thesaurus for Economics".to_string()), Some(url.to_string()))
    } else if url.contains("d-nb.info/gnd") {
        (Some("GND".to_string()), Some("https://d-nb.info/gnd/".to_string()))
    } else if url.contains("loc.gov") {
        (
            Some("LCSH".to_string()),
            Some("http://id.loc.gov/authorities/subjects".to_string()),
        )
    } else if url.contains("vocab.getty.edu") {
        (Some("AAT".to_string()), Some(url.to_string()))
    } else {
        (None, Some(url.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        extract_year, format_date_range, infer_subject_scheme, is_creator, license_identifier_to_label,
        map_contributor_type,
    };

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
        assert!(is_creator(&["Creator".to_string()]));
        assert!(is_creator(&["creator".to_string()]));
        assert!(!is_creator(&["Researcher".to_string()]));
        assert!(!is_creator(&["Data Collector".to_string()]));
        assert!(!is_creator(&["Contributor".to_string()]));
    }

    #[test]
    fn test_is_creator_multiple_types() {
        assert!(is_creator(&["Researcher".to_string(), "Project Leader".to_string()]));
        assert!(!is_creator(&["Researcher".to_string(), "Data Collector".to_string()]));
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
        assert_eq!(format_date_range("2020-01-01", "MISSING"), Some("2020-01-01".to_string()));
    }

    #[test]
    fn test_format_date_range_end_only() {
        assert_eq!(format_date_range("MISSING", "2023-12-31"), Some("2023-12-31".to_string()));
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
