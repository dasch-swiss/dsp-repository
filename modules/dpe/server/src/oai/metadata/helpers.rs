//! Shared helper functions for metadata transformation.

use std::collections::HashMap;

use dpe_web::domain::AccessRightsType;

/// Extracts a value from a multilingual HashMap, preferring English.
pub fn get_multilingual_value(map: &HashMap<String, String>) -> Option<String> {
    map.get("en").or_else(|| map.values().next()).cloned()
}

/// Extracts the year from a date string (YYYY-MM-DD or YYYY).
pub fn extract_year(date: &str) -> String {
    if date.len() >= 4 && date != "MISSING" {
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

/// Checks whether an attribution represents a creator (principal investigator
/// or project leader) using case-insensitive matching.
pub fn is_creator(contributor_types: &[String]) -> bool {
    contributor_types.iter().any(|t| {
        let lower = t.to_lowercase();
        lower == "project leader"
            || lower == "principal investigator (pi)"
            || lower == "author"
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
    let has_start = start != "MISSING" && !start.is_empty();
    let has_end = end != "MISSING" && !end.is_empty();
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
        "CC-BY-SA-4.0" => {
            "Creative Commons Attribution-ShareAlike 4.0 International".to_string()
        }
        "CC-BY-NC-4.0" => {
            "Creative Commons Attribution-NonCommercial 4.0 International".to_string()
        }
        "CC-BY-NC-SA-4.0" => {
            "Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International"
                .to_string()
        }
        "CC-BY-ND-4.0" => {
            "Creative Commons Attribution-NoDerivatives 4.0 International".to_string()
        }
        "CC-BY-NC-ND-4.0" => {
            "Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International"
                .to_string()
        }
        "CC0-1.0" => "Creative Commons Public Domain Dedication".to_string(),
        _ => identifier.to_string(),
    }
}

/// Infers a subject scheme from an AuthorityFileReference URL.
pub fn infer_subject_scheme(url: &str) -> (Option<String>, Option<String>) {
    if url.contains("skos.um.es") || url.contains("zbw.eu/stw") {
        (Some("STW Thesaurus for Economics".to_string()), Some(url.to_string()))
    } else if url.contains("d-nb.info/gnd") {
        (
            Some("GND".to_string()),
            Some("https://d-nb.info/gnd/".to_string()),
        )
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
