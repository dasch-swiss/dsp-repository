//! Metadata transformation for OAI-PMH records.
//!
//! This module handles the transformation of Research Projects into Dublin Core
//! and DataCite 4.6 metadata formats.

use app::domain::{AccessRightsType, Discipline, Project, TemporalCoverage};
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
    pub subjects: Vec<String>,
    pub descriptions: Vec<DataCiteDescription>,
    pub dates: Vec<DataCiteDate>,
    pub rights_list: Vec<DataCiteRights>,
}

#[derive(Debug, Default)]
pub struct DataCiteCreator {
    pub name: String,
    pub name_type: Option<String>,
}

#[derive(Debug, Default)]
pub struct DataCiteTitle {
    pub title: String,
    pub title_type: Option<String>,
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
pub struct DataCiteRights {
    pub rights: String,
    pub rights_uri: Option<String>,
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

const PUBLISHER: &str = "DaSCH - Swiss National Data and Service Center for the Humanities";
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
                    // AuthorityFileReference.text is Option<String>
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

        // dc:creator from attributions (project leaders/authors)
        for attr in &self.attributions {
            let is_leader = attr
                .contributor_type
                .iter()
                .any(|t| t == "Project leader" || t == "author" || t == "Author");
            if is_leader {
                record.creators.push(attr.contributor.clone());
            }
        }

        // dc:contributor from other attributions
        for attr in &self.attributions {
            let is_leader = attr
                .contributor_type
                .iter()
                .any(|t| t == "Project leader" || t == "author" || t == "Author");
            if !is_leader {
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
        record.resource_type = "Dataset".to_string();

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

        // dc:relation from publications
        if let Some(ref publications) = self.publications {
            for pub_info in publications {
                if let Some(ref pid) = pub_info.pid {
                    record.relations.push(pid.url.clone());
                }
            }
        }
        // Also add project URLs
        for url in &self.url {
            record.relations.push(url.clone());
        }

        // dc:coverage from temporal and spatial coverage
        for tc in &self.temporal_coverage {
            match tc {
                TemporalCoverage::Reference(ref_data) => {
                    // AuthorityFileReference.text is Option<String>
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

        // Creators (mandatory)
        for attr in &self.attributions {
            let is_leader = attr
                .contributor_type
                .iter()
                .any(|t| t == "Project leader" || t == "author" || t == "Author");
            if is_leader {
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

        // Title (mandatory)
        let title = if self.official_name != "MISSING" && !self.official_name.is_empty() {
            &self.official_name
        } else {
            &self.name
        };
        record.titles.push(DataCiteTitle {
            title: title.clone(),
            title_type: None,
            lang: Some("en".to_string()),
        });

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
        record.resource_type_general = "Other".to_string();

        // Subjects (recommended)
        for kw in &self.keywords {
            if let Some(keyword) = get_multilingual_value(kw) {
                record.subjects.push(keyword);
            }
        }

        // Descriptions (recommended)
        if let Some(ref abstract_map) = self.abstract_text {
            if let Some(abstract_text) = get_multilingual_value(abstract_map) {
                record.descriptions.push(DataCiteDescription {
                    description: abstract_text,
                    description_type: "Abstract".to_string(),
                    lang: Some("en".to_string()),
                });
            }
        }
        if let Some(desc) = get_multilingual_value(&self.description) {
            record.descriptions.push(DataCiteDescription {
                description: desc,
                description_type: "Other".to_string(),
                lang: Some("en".to_string()),
            });
        }

        // Dates
        if self.start_date != "MISSING" && !self.start_date.is_empty() {
            record.dates.push(DataCiteDate {
                date: self.start_date.clone(),
                date_type: "Created".to_string(),
            });
        }

        // Rights
        for legal in &self.legal_info {
            record.rights_list.push(DataCiteRights {
                rights: legal.license.license_identifier.clone(),
                rights_uri: if legal.license.license_uri != "MISSING" {
                    Some(legal.license.license_uri.clone())
                } else {
                    None
                },
            });
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
}
