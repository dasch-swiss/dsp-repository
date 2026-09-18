//! Transformation of Records into DataCite 4.6 metadata.

use crate::graph::RecordGraph;
use crate::types::{
    DataCiteCreator, DataCiteDate, DataCiteDescription, DataCiteRecord, DataCiteRelatedIdentifier, DataCiteRights,
    DataCiteTitle,
};

const PUBLISHER: &str = "DaSCH";

pub fn record_to_datacite(graph: &RecordGraph) -> DataCiteRecord {
    // Creators (mandatory) - from authorship, or the graph's organizational
    // fallback when the record carries none
    let creators: Vec<DataCiteCreator> = graph
        .creators_with_fallback()
        .iter()
        .map(|creator| DataCiteCreator {
            name: creator.name.clone(),
            name_type: Some(creator.kind.name_type().to_string()),
            ..Default::default()
        })
        .collect();

    // Titles (mandatory) - prefer "en", other languages as AlternativeTitles
    let mut titles: Vec<DataCiteTitle> = Vec::new();
    if let Some(ref title) = graph.title {
        titles.push(DataCiteTitle {
            title: title.clone(),
            title_type: None,
            lang: Some("en".to_string()),
        });
    }
    for (lang, alt_title) in &graph.alternative_titles {
        titles.push(DataCiteTitle {
            title: alt_title.clone(),
            title_type: Some("AlternativeTitle".to_string()),
            lang: Some(lang.clone()),
        });
    }

    // Dates (recommended)
    let mut dates: Vec<DataCiteDate> = Vec::new();
    if !graph.date_created.is_empty() {
        dates.push(DataCiteDate {
            date: graph.date_created.clone(),
            date_type: "Created".to_string(),
            ..Default::default()
        });
    }
    if !graph.date_modified.is_empty() {
        dates.push(DataCiteDate {
            date: graph.date_modified.clone(),
            date_type: "Updated".to_string(),
            ..Default::default()
        });
    }
    if !graph.date_published.is_empty() {
        dates.push(DataCiteDate {
            date: graph.date_published.clone(),
            date_type: "Available".to_string(),
            ..Default::default()
        });
    }

    // Descriptions (recommended)
    let mut descriptions: Vec<DataCiteDescription> = Vec::new();
    if let Some(ref desc) = graph.description {
        descriptions.push(DataCiteDescription {
            description: desc.clone(),
            description_type: "Abstract".to_string(),
            lang: None,
        });
    }
    // Size encoded as TechnicalInfo (DataCiteRecord has no dedicated sizes field)
    if !graph.size.is_empty() {
        descriptions.push(DataCiteDescription {
            description: graph.size.clone(),
            description_type: "TechnicalInfo".to_string(),
            lang: None,
        });
    }

    // Rights (optional)
    let has_identifier = !graph.license_identifier.is_empty();
    let rights_list = vec![DataCiteRights {
        rights: graph.license_label.clone().unwrap_or_else(|| graph.access_rights.clone()),
        rights_uri: if !graph.license_uri.is_empty() {
            Some(graph.license_uri.clone())
        } else {
            None
        },
        rights_identifier: if has_identifier {
            Some(graph.license_identifier.clone())
        } else {
            None
        },
        rights_identifier_scheme: if has_identifier { Some("SPDX".to_string()) } else { None },
    }];

    // RelatedIdentifiers — link to the parent project via IsPartOf. No HasPart for the file, and
    // no <sizes>: see docs/src/dpe/oai-pmh.md.
    let related_identifiers = vec![DataCiteRelatedIdentifier {
        identifier: graph.project_ark.clone(),
        related_identifier_type: "ARK".to_string(),
        relation_type: "IsPartOf".to_string(),
    }];

    // Formats (optional) — the file's MIME type (DataCite property 14, bitstream records only).
    let formats: Vec<String> = graph.mime_type.clone().into_iter().collect();

    DataCiteRecord {
        identifier: graph.ark.clone(),
        identifier_type: "ARK".to_string(),
        creators,
        titles,
        publisher: PUBLISHER.to_string(),
        publication_year: graph.publication_year.clone(),
        resource_type: graph.type_of_data.clone(),
        resource_type_general: graph.general_data_type.clone(),
        dates,
        descriptions,
        rights_list,
        related_identifiers,
        formats,
        ..DataCiteRecord::default()
    }
}

#[cfg(test)]
mod tests {
    use shared_metadata::record::Pid;
    use shared_metadata::utils::Multilingual;
    use shared_metadata::{Record, RecordFile, RecordLegalInfo, RecordLicense};

    use super::*;

    /// A record with a downloadable file (bitstream record).
    fn bitstream_record() -> Record {
        Record {
            file: Some(RecordFile {
                mime_type: Some("image/jp2".to_string()),
                url: "https://ingest.dasch.swiss/projects/0001/assets/5RMOnH7RmAY-qKzgr431bg7/original".to_string(),
                ..RecordFile::default()
            }),
            ..test_record()
        }
    }

    fn test_record() -> Record {
        Record {
            id: "record-0001".to_string(),
            pid: Pid::new("https://ark.dasch.swiss", "0001", "record-0001"),
            label: {
                let mut m = Multilingual::new();
                m.insert("en".to_string(), "Survey Responses on Rural Land Use, 1920–1950".to_string());
                m.insert(
                    "de".to_string(),
                    "Umfrageantworten zur ländlichen Landnutzung, 1920–1950".to_string(),
                );
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
                let mut m = Multilingual::new();
                m.insert("en".to_string(), "A collection of survey responses.".to_string());
                m
            },
            date_created: "2024-01-15".to_string(),
            date_modified: "2024-06-30".to_string(),
            date_published: "2024-02-01".to_string(),
            type_of_data: "Text".to_string(),
            size: "2.3 GB".to_string(),
            keywords: vec![],
            file: None,
        }
    }

    #[test]
    fn identifier_is_resolvable_ark_url() {
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
        assert_eq!(dc.identifier, "https://ark.dasch.swiss/ark:/72163/1/0001/record-0001");
        assert_eq!(dc.identifier_type, "ARK");
    }

    #[test]
    fn creators_from_authorship() {
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
        assert_eq!(dc.creators.len(), 2);
        assert_eq!(dc.creators[0].name, "Dr. Anna Müller");
        assert_eq!(dc.creators[1].name, "Prof. Hans Bauer");
    }

    #[test]
    fn personal_authorship_keeps_personal_name_type() {
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
        assert_eq!(dc.creators[0].name_type.as_deref(), Some("Personal"));
        assert_eq!(dc.creators[1].name_type.as_deref(), Some("Personal"));
    }

    #[test]
    fn empty_authorship_falls_back_to_organizational_dasch() {
        let mut record = test_record();
        record.legal_info.authorship = vec![];
        let dc = record_to_datacite(&RecordGraph::build(&record));
        assert_eq!(dc.creators.len(), 1);
        assert_eq!(dc.creators[0].name, "DaSCH");
        assert_eq!(dc.creators[0].name_type.as_deref(), Some("Organizational"));
    }

    #[test]
    fn title_prefers_english_as_primary() {
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
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
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
        assert_eq!(dc.publisher, "DaSCH");
    }

    #[test]
    fn publication_year_from_date_published() {
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
        assert_eq!(dc.publication_year, "2024");
    }

    #[test]
    fn resource_type_from_type_of_data() {
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
        assert_eq!(dc.resource_type, "Text");
        assert_eq!(dc.resource_type_general, "Text");
    }

    #[test]
    fn dates_include_created_updated_available() {
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
        let date_types: Vec<&str> = dc.dates.iter().map(|d| d.date_type.as_str()).collect();
        assert!(date_types.contains(&"Created"));
        assert!(date_types.contains(&"Updated"));
        assert!(date_types.contains(&"Available"));
    }

    #[test]
    fn rights_from_license() {
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
        assert_eq!(dc.rights_list.len(), 1);
        assert_eq!(dc.rights_list[0].rights, "Creative Commons Attribution 4.0 International");
        assert_eq!(dc.rights_list[0].rights_identifier.as_deref(), Some("CC-BY-4.0"));
        assert_eq!(dc.rights_list[0].rights_identifier_scheme.as_deref(), Some("SPDX"));
    }

    #[test]
    fn rights_fall_back_to_access_rights_without_a_license() {
        let mut record = test_record();
        record.legal_info.license = RecordLicense::default();
        let dc = record_to_datacite(&RecordGraph::build(&record));
        assert_eq!(dc.rights_list[0].rights, "Full Open Access");
        assert_eq!(dc.rights_list[0].rights_uri, None);
        assert_eq!(dc.rights_list[0].rights_identifier, None);
        assert_eq!(dc.rights_list[0].rights_identifier_scheme, None);
    }

    #[test]
    fn description_from_description_field() {
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
        let abstract_desc = dc.descriptions.iter().find(|d| d.description_type == "Abstract");
        assert!(abstract_desc.is_some());
        assert_eq!(abstract_desc.unwrap().description, "A collection of survey responses.");
    }

    #[test]
    fn size_included_as_technical_info() {
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
        let size_desc = dc.descriptions.iter().find(|d| d.description_type == "TechnicalInfo");
        assert!(size_desc.is_some());
        assert_eq!(size_desc.unwrap().description, "2.3 GB");
    }

    #[test]
    fn related_identifier_links_to_parent_project() {
        let mut record = test_record();
        record.pid = Pid::new("https://ark.dasch.swiss", "0803", "lklK7rVuVOmpBZYWrF8o=gh");
        let dc = record_to_datacite(&RecordGraph::build(&record));
        assert_eq!(dc.related_identifiers.len(), 1);
        let ri = &dc.related_identifiers[0];
        assert_eq!(ri.identifier, "https://ark.dasch.swiss/ark:/72163/1/0803");
        assert_eq!(ri.related_identifier_type, "ARK");
        assert_eq!(ri.relation_type, "IsPartOf");
    }

    #[test]
    fn record_without_file_has_no_format() {
        let dc = record_to_datacite(&RecordGraph::build(&test_record()));
        assert!(dc.formats.is_empty());
    }

    #[test]
    fn bitstream_format_is_mime_type() {
        let dc = record_to_datacite(&RecordGraph::build(&bitstream_record()));
        assert_eq!(dc.formats, vec!["image/jp2"]);
    }

    #[test]
    fn bitstream_file_url_is_not_a_related_identifier() {
        let record = bitstream_record();
        let dc = record_to_datacite(&RecordGraph::build(&record));

        assert!(!dc.related_identifiers.iter().any(|ri| ri.relation_type == "HasPart"));
        assert!(!dc.related_identifiers.iter().any(|ri| ri.identifier.contains("ingest.")));
        assert!(!dc.related_identifiers.iter().any(|ri| ri.identifier.contains("/dpe/records/")));

        assert_eq!(dc.related_identifiers.len(), 1);
        assert_eq!(dc.related_identifiers[0].relation_type, "IsPartOf");
        assert_eq!(dc.related_identifiers[0].identifier, record.project_ark());
    }

    /// `DataCiteRecord` has no size field; this guards the description leak path.
    #[test]
    fn file_size_does_not_reach_the_payload() {
        let record = Record {
            file: Some(RecordFile {
                mime_type: Some("image/png".to_string()),
                url: "https://ingest.dasch.swiss/projects/0001/assets/abc/original".to_string(),
                file_size: Some(377685),
                ..RecordFile::default()
            }),
            ..test_record()
        };
        let dc = record_to_datacite(&RecordGraph::build(&record));
        assert!(!dc.descriptions.iter().any(|d| d.description.contains("377685")));
    }
}
