//! Transformation of Records into DataCite 4.6 metadata.

use app::domain::Record;

use super::helpers::{extract_year, get_multilingual_value, license_identifier_to_label};
use super::types::{DataCiteCreator, DataCiteDate, DataCiteDescription, DataCiteRecord, DataCiteRelatedIdentifier, DataCiteRights, DataCiteTitle};

const PUBLISHER: &str = "DaSCH";

/// Extracts the ARK path from a full ARK URL.
/// "https://ark.dasch.swiss/ark:/72163/1/record-0001" -> "ark:/72163/1/record-0001"
fn ark_path_from_pid(pid: &str) -> String {
    if let Some(pos) = pid.find("ark:/") {
        pid[pos..].to_string()
    } else {
        pid.to_string()
    }
}

/// Maps typeOfData to a DataCite resourceTypeGeneral value.
fn type_of_data_to_general(type_of_data: &str) -> String {
    match type_of_data {
        "Image" => "Image".to_string(),
        "Text" | "XML (TEI)" => "Text".to_string(),
        "Video" => "Audiovisual".to_string(),
        "Audio" => "Sound".to_string(),
        other => other.to_string(),
    }
}

pub fn record_to_datacite(record: &Record) -> DataCiteRecord {
    // Creators (mandatory) - from authorship
    let mut creators: Vec<DataCiteCreator> = record
        .legal_info
        .authorship
        .iter()
        .map(|name| DataCiteCreator { name: name.clone(), name_type: Some("Personal".to_string()) })
        .collect();
    if creators.is_empty() {
        creators.push(DataCiteCreator {
            name: PUBLISHER.to_string(),
            name_type: Some("Organizational".to_string()),
        });
    }

    // Titles (mandatory) - prefer "en", other languages as AlternativeTitles
    let mut titles: Vec<DataCiteTitle> = Vec::new();
    if let Some(title) = get_multilingual_value(&record.label) {
        titles.push(DataCiteTitle { title, title_type: None, lang: Some("en".to_string()) });
    }
    let mut lang_keys: Vec<&String> = record.label.keys().collect();
    lang_keys.sort();
    for lang in lang_keys {
        if lang != "en" {
            if let Some(alt_title) = record.label.get(lang) {
                titles.push(DataCiteTitle {
                    title: alt_title.clone(),
                    title_type: Some("AlternativeTitle".to_string()),
                    lang: Some(lang.clone()),
                });
            }
        }
    }

    // Dates (recommended)
    let mut dates: Vec<DataCiteDate> = Vec::new();
    if !record.date_created.is_empty() {
        dates.push(DataCiteDate { date: record.date_created.clone(), date_type: "Created".to_string() });
    }
    if !record.date_modified.is_empty() {
        dates.push(DataCiteDate { date: record.date_modified.clone(), date_type: "Updated".to_string() });
    }
    if !record.date_published.is_empty() {
        dates.push(DataCiteDate { date: record.date_published.clone(), date_type: "Available".to_string() });
    }

    // Descriptions (recommended)
    let mut descriptions: Vec<DataCiteDescription> = Vec::new();
    if let Some(desc) = get_multilingual_value(&record.description) {
        descriptions.push(DataCiteDescription { description: desc, description_type: "Abstract".to_string(), lang: None });
    }
    // Size encoded as TechnicalInfo (DataCiteRecord has no dedicated sizes field)
    if !record.size.is_empty() {
        descriptions.push(DataCiteDescription {
            description: record.size.clone(),
            description_type: "TechnicalInfo".to_string(),
            lang: None,
        });
    }

    // Rights (optional)
    let license = &record.legal_info.license;
    let has_identifier = !license.license_identifier.is_empty();
    let rights_list = vec![DataCiteRights {
        rights: if has_identifier {
            license_identifier_to_label(&license.license_identifier)
        } else {
            record.access_rights.clone()
        },
        rights_uri: if !license.license_uri.is_empty() { Some(license.license_uri.clone()) } else { None },
        rights_identifier: if has_identifier { Some(license.license_identifier.clone()) } else { None },
        rights_identifier_scheme: if has_identifier { Some("SPDX".to_string()) } else { None },
    }];

    // RelatedIdentifiers — link to parent project via IsPartOf
    let related_identifiers = record
        .project_ark()
        .map(|ark| vec![DataCiteRelatedIdentifier {
            identifier: ark,
            related_identifier_type: "URL".to_string(),
            relation_type: "IsPartOf".to_string(),
        }])
        .unwrap_or_default();

    DataCiteRecord {
        identifier: ark_path_from_pid(&record.pid),
        identifier_type: "ARK".to_string(),
        creators,
        titles,
        publisher: PUBLISHER.to_string(),
        publication_year: extract_year(&record.date_published),
        resource_type: record.type_of_data.clone(),
        resource_type_general: type_of_data_to_general(&record.type_of_data),
        dates,
        descriptions,
        rights_list,
        related_identifiers,
        ..DataCiteRecord::default()
    }
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
    fn identifier_is_ark_path() {
        let dc = record_to_datacite(&test_record());
        assert_eq!(dc.identifier, "ark:/72163/1/record-0001");
        assert_eq!(dc.identifier_type, "ARK");
    }

    #[test]
    fn creators_from_authorship() {
        let dc = record_to_datacite(&test_record());
        assert_eq!(dc.creators.len(), 2);
        assert_eq!(dc.creators[0].name, "Dr. Anna Müller");
        assert_eq!(dc.creators[1].name, "Prof. Hans Bauer");
    }

    #[test]
    fn title_prefers_english_as_primary() {
        let dc = record_to_datacite(&test_record());
        assert!(!dc.titles.is_empty());
        assert_eq!(dc.titles[0].title, "Survey Responses on Rural Land Use, 1920–1950");
        assert_eq!(dc.titles[0].title_type, None);
        // German should appear as AlternativeTitle
        let alt = dc.titles.iter().find(|t| t.title_type.as_deref() == Some("AlternativeTitle"));
        assert!(alt.is_some());
        assert_eq!(alt.unwrap().title, "Umfrageantworten zur ländlichen Landnutzung, 1920–1950");
    }

    #[test]
    fn publisher_is_dasch() {
        let dc = record_to_datacite(&test_record());
        assert_eq!(dc.publisher, "DaSCH");
    }

    #[test]
    fn publication_year_from_date_published() {
        let dc = record_to_datacite(&test_record());
        assert_eq!(dc.publication_year, "2024");
    }

    #[test]
    fn resource_type_from_type_of_data() {
        let dc = record_to_datacite(&test_record());
        assert_eq!(dc.resource_type, "Text");
        assert_eq!(dc.resource_type_general, "Text");
    }

    #[test]
    fn dates_include_created_updated_available() {
        let dc = record_to_datacite(&test_record());
        let date_types: Vec<&str> = dc.dates.iter().map(|d| d.date_type.as_str()).collect();
        assert!(date_types.contains(&"Created"));
        assert!(date_types.contains(&"Updated"));
        assert!(date_types.contains(&"Available"));
    }

    #[test]
    fn rights_from_license() {
        let dc = record_to_datacite(&test_record());
        assert_eq!(dc.rights_list.len(), 1);
        assert_eq!(dc.rights_list[0].rights, "Creative Commons Attribution 4.0 International");
        assert_eq!(dc.rights_list[0].rights_identifier.as_deref(), Some("CC-BY-4.0"));
        assert_eq!(dc.rights_list[0].rights_identifier_scheme.as_deref(), Some("SPDX"));
    }

    #[test]
    fn description_from_description_field() {
        let dc = record_to_datacite(&test_record());
        let abstract_desc = dc.descriptions.iter().find(|d| d.description_type == "Abstract");
        assert!(abstract_desc.is_some());
        assert_eq!(abstract_desc.unwrap().description, "A collection of survey responses.");
    }

    #[test]
    fn size_included_as_technical_info() {
        let dc = record_to_datacite(&test_record());
        let size_desc = dc.descriptions.iter().find(|d| d.description_type == "TechnicalInfo");
        assert!(size_desc.is_some());
        assert_eq!(size_desc.unwrap().description, "2.3 GB");
    }

    #[test]
    fn type_of_data_to_general_mappings() {
        assert_eq!(type_of_data_to_general("Image"), "Image");
        assert_eq!(type_of_data_to_general("Text"), "Text");
        assert_eq!(type_of_data_to_general("XML (TEI)"), "Text");
        assert_eq!(type_of_data_to_general("Video"), "Audiovisual");
        assert_eq!(type_of_data_to_general("Audio"), "Sound");
    }

    #[test]
    fn related_identifier_links_to_parent_project() {
        let mut record = test_record();
        record.pid = "https://ark.dasch.swiss/ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh".to_string();
        let dc = record_to_datacite(&record);
        assert_eq!(dc.related_identifiers.len(), 1);
        let ri = &dc.related_identifiers[0];
        assert_eq!(ri.identifier, "https://ark.dasch.swiss/ark:/72163/1/0803");
        assert_eq!(ri.related_identifier_type, "URL");
        assert_eq!(ri.relation_type, "IsPartOf");
    }

    #[test]
    fn no_related_identifier_when_pid_has_no_parent() {
        // single-segment ARK — no parent project
        let dc = record_to_datacite(&test_record());
        assert!(dc.related_identifiers.is_empty());
    }

    #[test]
    fn ark_path_from_pid_extracts_ark() {
        assert_eq!(
            ark_path_from_pid("https://ark.dasch.swiss/ark:/72163/1/record-0001"),
            "ark:/72163/1/record-0001"
        );
    }
}
