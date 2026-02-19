//! Metadata transformation for OAI-PMH records.
//!
//! This module handles the transformation of Research Projects into Dublin Core
//! and DataCite 4.6 metadata formats, following the DaSCH Metadata to DataCite
//! mapping specification.

use app::domain::{AccessRightsType, Discipline, Funding, Project, TemporalCoverage};
use std::collections::HashMap;

/// Dublin Core metadata record containing the 15 DC elements.
#[derive(Debug, Default)]
pub struct DublinCoreRecord {
    pub titles: Vec<String>,
    pub creators: Vec<String>,
    pub subjects: Vec<String>,
    pub descriptions: Vec<String>,
    pub publisher: String,
    pub contributors: Vec<String>,
    pub dates: Vec<String>,
    pub resource_type: String,
    pub identifiers: Vec<String>,
    pub languages: Vec<String>,
    pub relations: Vec<String>,
    pub coverages: Vec<String>,
    pub rights: Vec<String>,
}

/// DataCite 4.6 metadata record containing mandatory and recommended properties.
#[derive(Debug, Default)]
pub struct DataCiteRecord {
    pub identifier: String,
    pub identifier_type: String,
    pub creators: Vec<DataCiteCreator>,
    pub titles: Vec<DataCiteTitle>,
    pub publisher: String,
    pub publication_year: String,
    pub resource_type: String,
    pub resource_type_general: String,
    pub subjects: Vec<DataCiteSubject>,
    pub contributors: Vec<DataCiteContributor>,
    pub descriptions: Vec<DataCiteDescription>,
    pub dates: Vec<DataCiteDate>,
    pub language: Option<String>,
    pub related_identifiers: Vec<DataCiteRelatedIdentifier>,
    pub rights_list: Vec<DataCiteRights>,
    pub geo_locations: Vec<DataCiteGeoLocation>,
    pub funding_references: Vec<DataCiteFundingReference>,
}

#[derive(Debug, Default)]
pub struct DataCiteCreator {
    pub name: String,
    pub name_type: Option<String>,
}

#[derive(Debug, Default)]
pub struct DataCiteContributor {
    pub name: String,
    pub name_type: Option<String>,
    pub contributor_type: String,
}

#[derive(Debug, Default)]
pub struct DataCiteTitle {
    pub title: String,
    pub title_type: Option<String>,
    pub lang: Option<String>,
}

#[derive(Debug, Default)]
pub struct DataCiteSubject {
    pub subject: String,
    pub subject_scheme: Option<String>,
    pub scheme_uri: Option<String>,
    pub lang: Option<String>,
}

#[derive(Debug, Default)]
pub struct DataCiteDescription {
    pub description: String,
    pub description_type: String,
    pub lang: Option<String>,
}

#[derive(Debug, Default)]
pub struct DataCiteDate {
    pub date: String,
    pub date_type: String,
}

#[derive(Debug, Default)]
pub struct DataCiteRelatedIdentifier {
    pub identifier: String,
    pub related_identifier_type: String,
    pub relation_type: String,
}

#[derive(Debug, Default)]
pub struct DataCiteRights {
    pub rights: String,
    pub rights_uri: Option<String>,
    pub rights_identifier: Option<String>,
    pub rights_identifier_scheme: Option<String>,
}

#[derive(Debug, Default)]
pub struct DataCiteGeoLocation {
    pub geo_location_place: String,
}

#[derive(Debug, Default)]
pub struct DataCiteFundingReference {
    pub funder_name: String,
    pub award_number: Option<String>,
    pub award_title: Option<String>,
    pub award_uri: Option<String>,
}

/// OAI-PMH record header containing identifier and datestamp.
#[derive(Debug)]
pub struct OaiRecordHeader {
    pub identifier: String,
    pub datestamp: String,
    pub set_specs: Vec<String>,
}

/// Complete OAI-PMH record with header and metadata.
#[derive(Debug)]
pub struct OaiRecord {
    pub header: OaiRecordHeader,
    pub dublin_core: Option<DublinCoreRecord>,
    pub datacite: Option<DataCiteRecord>,
}

const PUBLISHER: &str = "DaSCH";
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

/// Extracts a value from a multilingual HashMap, preferring English.
fn get_multilingual_value(map: &HashMap<String, String>) -> Option<String> {
    map.get("en").or_else(|| map.values().next()).cloned()
}

/// Extracts the year from a date string (YYYY-MM-DD or YYYY).
fn extract_year(date: &str) -> String {
    if date.len() >= 4 && date != "MISSING" {
        date[..4].to_string()
    } else {
        "2015".to_string() // Default fallback year
    }
}

/// Converts an AccessRightsType to a human-readable string.
fn access_rights_to_string(ar: &AccessRightsType) -> &'static str {
    match ar {
        AccessRightsType::FullOpenAccess => "Full Open Access",
        AccessRightsType::OpenAccessWithRestrictions => "Open Access with Restrictions",
        AccessRightsType::EmbargoedAccess => "Embargoed Access",
        AccessRightsType::MetadataOnlyAccess => "Metadata only Access",
    }
}

/// Checks whether an attribution represents a creator (principal investigator
/// or project leader) using case-insensitive matching.
fn is_creator(contributor_types: &[String]) -> bool {
    contributor_types.iter().any(|t| {
        let lower = t.to_lowercase();
        lower == "project leader"
            || lower == "principal investigator (pi)"
            || lower == "author"
    })
}

/// Maps a contributor type string to the closest DataCite contributorType
/// vocabulary term.
fn map_contributor_type(contributor_type: &str) -> &'static str {
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
fn format_date_range(start: &str, end: &str) -> Option<String> {
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
fn license_identifier_to_label(identifier: &str) -> String {
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
fn infer_subject_scheme(url: &str) -> (Option<String>, Option<String>) {
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

/// Extension trait for Project to provide OAI-PMH metadata conversion methods.
pub trait ProjectOaiExt {
    /// Converts this project to a Dublin Core record.
    fn to_dublin_core(&self) -> DublinCoreRecord;

    /// Converts this project to a DataCite 4.6 record.
    fn to_datacite(&self) -> DataCiteRecord;

    /// Creates an OAI record from this project.
    fn to_oai_record(&self, metadata_prefix: &str) -> OaiRecord;

    /// Checks if this project matches the given date filter.
    fn matches_date_filter(&self, from: Option<&str>, until: Option<&str>) -> bool;
}

impl ProjectOaiExt for Project {
    fn to_dublin_core(&self) -> DublinCoreRecord {
        let mut record = DublinCoreRecord::default();

        // dc:title - prefer officialName, fallback to name
        let title = if self.official_name != "MISSING" && !self.official_name.is_empty() {
            &self.official_name
        } else {
            &self.name
        };
        record.titles.push(title.clone());

        // dc:title - additional alternative names
        if let Some(ref alt_names) = self.alternative_names {
            for alt_name_map in alt_names {
                if let Some(alt_name) = get_multilingual_value(alt_name_map) {
                    if !record.titles.contains(&alt_name) {
                        record.titles.push(alt_name);
                    }
                }
            }
        }

        // dc:description - prefer English from description field
        if let Some(desc) = get_multilingual_value(&self.description) {
            record.descriptions.push(desc);
        }
        // Also include abstract if available
        if let Some(ref abstract_map) = self.abstract_text {
            if let Some(abstract_text) = get_multilingual_value(abstract_map) {
                if !record.descriptions.contains(&abstract_text) {
                    record.descriptions.push(abstract_text);
                }
            }
        }

        // dc:subject from keywords
        for kw in &self.keywords {
            if let Some(keyword) = get_multilingual_value(kw) {
                record.subjects.push(keyword);
            }
        }

        // dc:subject from disciplines
        for discipline in &self.disciplines {
            match discipline {
                Discipline::Reference(ref_data) => {
                    if let Some(ref text) = ref_data.text {
                        record.subjects.push(text.clone());
                    }
                }
                Discipline::Text(text_map) => {
                    if let Some(text) = get_multilingual_value(text_map) {
                        record.subjects.push(text);
                    }
                }
            }
        }

        // dc:creator from attributions (principal investigators and project leaders)
        for attr in &self.attributions {
            if is_creator(&attr.contributor_type) {
                record.creators.push(attr.contributor.clone());
            }
        }

        // dc:contributor from other attributions
        for attr in &self.attributions {
            if !is_creator(&attr.contributor_type) {
                record.contributors.push(attr.contributor.clone());
            }
        }

        // dc:publisher
        record.publisher = PUBLISHER.to_string();

        // dc:date from startDate
        if self.start_date != "MISSING" && !self.start_date.is_empty() {
            record.dates.push(self.start_date.clone());
        }

        // dc:type
        record.resource_type = "Project".to_string();

        // dc:identifier - use PID or shortcode
        if self.pid != "MISSING" && !self.pid.is_empty() {
            record.identifiers.push(self.pid.clone());
        }
        record.identifiers.push(make_oai_identifier(&self.shortcode));

        // dc:language
        if let Some(ref languages) = self.data_language {
            for lang_map in languages {
                if let Some(lang) = get_multilingual_value(lang_map) {
                    record.languages.push(lang);
                }
            }
        }

        // dc:relation -- should be the parent Project Cluster ARK.
        // TODO: Populate once Project Cluster data is available.

        // dc:coverage from temporal and spatial coverage
        for tc in &self.temporal_coverage {
            match tc {
                TemporalCoverage::Reference(ref_data) => {
                    if let Some(ref text) = ref_data.text {
                        record.coverages.push(text.clone());
                    }
                }
                TemporalCoverage::Text(text_map) => {
                    if let Some(text) = get_multilingual_value(text_map) {
                        record.coverages.push(text);
                    }
                }
            }
        }
        for sc in &self.spatial_coverage {
            if let Some(ref text) = sc.text {
                record.coverages.push(text.clone());
            }
        }

        // dc:rights
        record
            .rights
            .push(access_rights_to_string(&self.access_rights.access_rights).to_string());
        for legal in &self.legal_info {
            if legal.license.license_uri != "MISSING" && !legal.license.license_uri.is_empty() {
                record.rights.push(legal.license.license_uri.clone());
            }
        }

        record
    }

    fn to_datacite(&self) -> DataCiteRecord {
        let mut record = DataCiteRecord::default();

        // Identifier (mandatory) - use PID or generate from shortcode
        if self.pid != "MISSING" && !self.pid.is_empty() {
            record.identifier = self.pid.clone();
            record.identifier_type = "ARK".to_string();
        } else {
            record.identifier = format!("ark:/72163/1/{}", self.shortcode);
            record.identifier_type = "ARK".to_string();
        }

        // Creators (mandatory) - principal investigators and project leaders
        for attr in &self.attributions {
            if is_creator(&attr.contributor_type) {
                record.creators.push(DataCiteCreator {
                    name: attr.contributor.clone(),
                    name_type: Some("Personal".to_string()),
                });
            }
        }
        // Ensure at least one creator
        if record.creators.is_empty() {
            record.creators.push(DataCiteCreator {
                name: "DaSCH".to_string(),
                name_type: Some("Organizational".to_string()),
            });
        }

        // Contributors - non-creator attributions mapped to DataCite vocabulary
        for attr in &self.attributions {
            if !is_creator(&attr.contributor_type) {
                let datacite_type = attr
                    .contributor_type
                    .first()
                    .map(|t| map_contributor_type(t))
                    .unwrap_or("Other");
                record.contributors.push(DataCiteContributor {
                    name: attr.contributor.clone(),
                    name_type: Some("Personal".to_string()),
                    contributor_type: datacite_type.to_string(),
                });
            }
        }

        // Titles (mandatory)
        // Use the longer of name/officialName as primary, shorter as AlternativeTitle
        let name_valid =
            self.name != "MISSING" && !self.name.is_empty();
        let official_valid =
            self.official_name != "MISSING" && !self.official_name.is_empty();

        match (name_valid, official_valid) {
            (true, true) => {
                let (primary, alternative) = if self.official_name.len() >= self.name.len() {
                    (&self.official_name, &self.name)
                } else {
                    (&self.name, &self.official_name)
                };
                record.titles.push(DataCiteTitle {
                    title: primary.clone(),
                    title_type: None,
                    lang: None,
                });
                if primary != alternative {
                    record.titles.push(DataCiteTitle {
                        title: alternative.clone(),
                        title_type: Some("AlternativeTitle".to_string()),
                        lang: None,
                    });
                }
            }
            (false, true) => {
                record.titles.push(DataCiteTitle {
                    title: self.official_name.clone(),
                    title_type: None,
                    lang: None,
                });
            }
            _ => {
                record.titles.push(DataCiteTitle {
                    title: self.name.clone(),
                    title_type: None,
                    lang: None,
                });
            }
        }

        // Additional alternative names
        if let Some(ref alt_names) = self.alternative_names {
            for alt_name_map in alt_names {
                if let Some(alt_name) = get_multilingual_value(alt_name_map) {
                    let already_present = record.titles.iter().any(|t| t.title == alt_name);
                    if !already_present {
                        record.titles.push(DataCiteTitle {
                            title: alt_name,
                            title_type: Some("AlternativeTitle".to_string()),
                            lang: None,
                        });
                    }
                }
            }
        }

        // Publisher (mandatory)
        record.publisher = PUBLISHER.to_string();

        // PublicationYear (mandatory)
        if let Some(ref pub_year) = self.data_publication_year {
            record.publication_year = extract_year(pub_year);
        } else {
            record.publication_year = extract_year(&self.start_date);
        }

        // ResourceType (mandatory)
        record.resource_type = "Research Project".to_string();
        record.resource_type_general = "Project".to_string();

        // Subjects (recommended) - keywords without scheme info
        for kw in &self.keywords {
            if let Some(keyword) = get_multilingual_value(kw) {
                record.subjects.push(DataCiteSubject {
                    subject: keyword,
                    subject_scheme: None,
                    scheme_uri: None,
                    lang: None,
                });
            }
        }

        // Subjects from disciplines - with scheme info when available
        for discipline in &self.disciplines {
            match discipline {
                Discipline::Reference(ref_data) => {
                    if let Some(ref text) = ref_data.text {
                        let (scheme, scheme_uri) = infer_subject_scheme(&ref_data.url);
                        record.subjects.push(DataCiteSubject {
                            subject: text.clone(),
                            subject_scheme: scheme,
                            scheme_uri,
                            lang: None,
                        });
                    }
                }
                Discipline::Text(text_map) => {
                    if let Some(text) = get_multilingual_value(text_map) {
                        record.subjects.push(DataCiteSubject {
                            subject: text,
                            subject_scheme: None,
                            scheme_uri: None,
                            lang: None,
                        });
                    }
                }
            }
        }

        // Descriptions (recommended)
        if let Some(ref abstract_map) = self.abstract_text {
            if let Some(abstract_text) = get_multilingual_value(abstract_map) {
                record.descriptions.push(DataCiteDescription {
                    description: abstract_text,
                    description_type: "Abstract".to_string(),
                    lang: None,
                });
            }
        }
        if let Some(desc) = get_multilingual_value(&self.description) {
            record.descriptions.push(DataCiteDescription {
                description: desc,
                description_type: "Other".to_string(),
                lang: None,
            });
        }

        // Dates - use startDate/endDate range as dateType="Collected"
        if let Some(date_range) = format_date_range(&self.start_date, &self.end_date) {
            record.dates.push(DataCiteDate {
                date: date_range,
                date_type: "Collected".to_string(),
            });
        }

        // Dates - temporal coverage as dateType="Coverage"
        for tc in &self.temporal_coverage {
            let coverage_text = match tc {
                TemporalCoverage::Reference(ref_data) => ref_data.text.clone(),
                TemporalCoverage::Text(text_map) => get_multilingual_value(text_map),
            };
            if let Some(text) = coverage_text {
                record.dates.push(DataCiteDate {
                    date: text,
                    date_type: "Coverage".to_string(),
                });
            }
        }

        // Language - from data_language (use English value as-is since we don't
        // have ISO codes in the data)
        if let Some(ref languages) = self.data_language {
            if let Some(first_lang) = languages.first() {
                if let Some(lang_value) = get_multilingual_value(first_lang) {
                    record.language = Some(lang_value);
                }
            }
        }

        // RelatedIdentifiers -- should contain parent Project Cluster ARK.
        // TODO: Populate once Project Cluster data is available.

        // Rights - with SPDX identifier
        for legal in &self.legal_info {
            let rights_uri = if legal.license.license_uri != "MISSING" {
                Some(legal.license.license_uri.clone())
            } else {
                None
            };
            let has_identifier = legal.license.license_identifier != "MISSING"
                && !legal.license.license_identifier.is_empty();
            record.rights_list.push(DataCiteRights {
                rights: if has_identifier {
                    license_identifier_to_label(&legal.license.license_identifier)
                } else {
                    access_rights_to_string(&self.access_rights.access_rights).to_string()
                },
                rights_uri,
                rights_identifier: if has_identifier {
                    Some(legal.license.license_identifier.clone())
                } else {
                    None
                },
                rights_identifier_scheme: if has_identifier {
                    Some("SPDX".to_string())
                } else {
                    None
                },
            });
        }

        // GeoLocations from spatial_coverage
        for sc in &self.spatial_coverage {
            if let Some(ref text) = sc.text {
                record.geo_locations.push(DataCiteGeoLocation {
                    geo_location_place: text.clone(),
                });
            }
        }

        // FundingReferences from grants
        // Note: funder names are currently internal IDs (e.g., "0801-organization-000"),
        // not human-readable names. This is a data quality issue to resolve upstream.
        if let Funding::Grants(ref grants) = self.funding {
            for grant in grants {
                for funder in &grant.funders {
                    record.funding_references.push(DataCiteFundingReference {
                        funder_name: funder.clone(),
                        award_number: grant.number.clone(),
                        award_title: grant.name.clone(),
                        award_uri: grant.url.clone(),
                    });
                }
            }
        }

        record
    }

    fn to_oai_record(&self, metadata_prefix: &str) -> OaiRecord {
        let header = OaiRecordHeader {
            identifier: make_oai_identifier(&self.shortcode),
            // Use startDate as fallback for metadataModified
            datestamp: if self.start_date != "MISSING" && !self.start_date.is_empty() {
                self.start_date.clone()
            } else {
                "2015-01-01".to_string()
            },
            set_specs: vec!["entityType:ResearchProject".to_string()],
        };

        let dublin_core = if metadata_prefix == "oai_dc" {
            Some(self.to_dublin_core())
        } else {
            None
        };

        let datacite = if metadata_prefix == "oai_datacite" {
            Some(self.to_datacite())
        } else {
            None
        };

        OaiRecord { header, dublin_core, datacite }
    }

    fn matches_date_filter(&self, from: Option<&str>, until: Option<&str>) -> bool {
        // Use startDate as proxy for metadataModified
        let datestamp = if self.start_date != "MISSING" && !self.start_date.is_empty() {
            &self.start_date
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            license_identifier_to_label("UNKNOWN"),
            "UNKNOWN"
        );
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
