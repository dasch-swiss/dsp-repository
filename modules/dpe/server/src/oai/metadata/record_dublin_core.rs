//! Transformation of Records into Dublin Core metadata.

use app::domain::Record;

use super::helpers::{get_multilingual_value, license_identifier_to_label};
use super::types::DublinCoreRecord;

const PUBLISHER: &str = "DaSCH";

/// Maps typeOfData to a Dublin Core dc:type value.
fn type_of_data_to_dc_type(type_of_data: &str) -> String {
    match type_of_data {
        "Image" => "Image".to_string(),
        "Text" | "XML (TEI)" => "Text".to_string(),
        "Video" => "Audiovisual".to_string(),
        "Audio" => "Sound".to_string(),
        other => other.to_string(),
    }
}

pub fn record_to_dublin_core(record: &Record) -> DublinCoreRecord {
    let mut dc = DublinCoreRecord::default();

    // dc:identifier - use pid
    dc.identifiers.push(record.pid.clone());

    // dc:title - prefer "en", fallback to first available
    if let Some(title) = get_multilingual_value(&record.label) {
        dc.titles.push(title);
    }

    // dc:creator from authorship
    dc.creators = record.legal_info.authorship.clone();

    // dc:description - prefer "en", fallback to first available
    if let Some(desc) = get_multilingual_value(&record.description) {
        dc.descriptions.push(desc);
    }

    // dc:publisher
    dc.publisher = PUBLISHER.to_string();

    // dc:date from datePublished
    if !record.date_published.is_empty() {
        dc.dates.push(record.date_published.clone());
    }

    // dc:type derived from typeOfData
    dc.resource_type = type_of_data_to_dc_type(&record.type_of_data);

    // dc:relation — link to parent project
    if let Some(project_ark) = record.project_ark() {
        dc.relations.push(project_ark);
    }

    // dc:rights - license identifier and URI
    let id = &record.legal_info.license.license_identifier;
    if !id.is_empty() {
        dc.rights.push(license_identifier_to_label(id));
    }
    let uri = &record.legal_info.license.license_uri;
    if !uri.is_empty() {
        dc.rights.push(uri.clone());
    }

    dc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use app::domain::{RecordLegalInfo, RecordLicense};

    fn test_record() -> Record {
        Record {
            id: "record-0001".to_string(),
            pid: "https://ark.dasch.swiss/ark:/72163/1/record-0001".to_string(),
            label: {
                let mut m = HashMap::new();
                m.insert("en".to_string(), "Survey Responses on Rural Land Use, 1920–1950".to_string());
                m.insert("de".to_string(), "Umfrageantworten zur ländlichen Landnutzung, 1920–1950".to_string());
                m
            },
            access_rights: "Full Open Access".to_string(),
            legal_info: RecordLegalInfo {
                license: RecordLicense {
                    license_identifier: "CC-BY-4.0".to_string(),
                    license_date: "2024-01-15".to_string(),
                    license_uri: "https://creativecommons.org/licenses/by/4.0/".to_string(),
                },
                copyright_holder: "University of Basel".to_string(),
                authorship: vec!["Dr. Anna Müller".to_string(), "Prof. Hans Bauer".to_string()],
            },
            how_to_cite: String::new(),
            publisher: "DaSCH".to_string(),
            source: String::new(),
            description: {
                let mut m = HashMap::new();
                m.insert("en".to_string(), "A collection of survey responses.".to_string());
                m
            },
            date_created: "2024-01-15".to_string(),
            date_modified: "2024-06-30".to_string(),
            date_published: "2024-02-01".to_string(),
            type_of_data: "Text".to_string(),
            size: "2.3 GB".to_string(),
            keywords: vec![],
        }
    }

    #[test]
    fn identifier_is_pid() {
        let dc = record_to_dublin_core(&test_record());
        assert!(dc.identifiers.contains(&"https://ark.dasch.swiss/ark:/72163/1/record-0001".to_string()));
    }

    #[test]
    fn title_prefers_english() {
        let dc = record_to_dublin_core(&test_record());
        assert_eq!(dc.titles, vec!["Survey Responses on Rural Land Use, 1920–1950"]);
    }

    #[test]
    fn creators_from_authorship() {
        let dc = record_to_dublin_core(&test_record());
        assert_eq!(dc.creators, vec!["Dr. Anna Müller", "Prof. Hans Bauer"]);
    }

    #[test]
    fn description_in_english() {
        let dc = record_to_dublin_core(&test_record());
        assert_eq!(dc.descriptions, vec!["A collection of survey responses."]);
    }

    #[test]
    fn publisher_is_dasch() {
        let dc = record_to_dublin_core(&test_record());
        assert_eq!(dc.publisher, "DaSCH");
    }

    #[test]
    fn date_is_date_published() {
        let dc = record_to_dublin_core(&test_record());
        assert!(dc.dates.contains(&"2024-02-01".to_string()));
    }

    #[test]
    fn resource_type_mapped_from_type_of_data() {
        let dc = record_to_dublin_core(&test_record());
        assert_eq!(dc.resource_type, "Text");
    }

    #[test]
    fn relation_links_to_parent_project() {
        let mut record = test_record();
        record.pid = "https://ark.dasch.swiss/ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh".to_string();
        let dc = record_to_dublin_core(&record);
        assert!(dc.relations.contains(&"https://ark.dasch.swiss/ark:/72163/1/0803".to_string()));
    }

    #[test]
    fn no_relation_when_pid_has_no_parent() {
        // single-segment ARK — no parent project
        let dc = record_to_dublin_core(&test_record());
        assert!(dc.relations.is_empty());
    }

    #[test]
    fn rights_contains_license_label_and_uri() {
        let dc = record_to_dublin_core(&test_record());
        assert!(dc.rights.contains(&"Creative Commons Attribution 4.0 International".to_string()));
        assert!(dc.rights.contains(&"https://creativecommons.org/licenses/by/4.0/".to_string()));
    }

    #[test]
    fn type_of_data_to_dc_type_mappings() {
        assert_eq!(type_of_data_to_dc_type("Image"), "Image");
        assert_eq!(type_of_data_to_dc_type("Text"), "Text");
        assert_eq!(type_of_data_to_dc_type("XML (TEI)"), "Text");
        assert_eq!(type_of_data_to_dc_type("Video"), "Audiovisual");
        assert_eq!(type_of_data_to_dc_type("Audio"), "Sound");
        assert_eq!(type_of_data_to_dc_type("Other"), "Other");
    }
}
