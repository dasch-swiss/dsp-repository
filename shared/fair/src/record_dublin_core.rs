//! Transformation of Records into Dublin Core metadata.

#[cfg(test)]
use crate::graph::ArkHost;
use crate::graph::RecordGraph;
use crate::types::DublinCoreRecord;

const PUBLISHER: &str = "DaSCH";

pub fn record_to_dublin_core(graph: &RecordGraph) -> DublinCoreRecord {
    let mut dc = DublinCoreRecord::default();

    // The ARK only — no file download URL: see docs/src/dpe/oai-pmh.md.
    dc.identifiers.push(graph.ark.clone());

    // dc:title - prefer "en", fallback to first available
    if let Some(ref title) = graph.title {
        dc.titles.push(title.clone());
    }

    // dc:creator from authorship
    dc.creators = graph.creators.iter().map(|creator| creator.name.clone()).collect();

    // dc:description - prefer "en", fallback to first available
    if let Some(ref desc) = graph.description {
        dc.descriptions.push(desc.clone());
    }

    // dc:publisher
    dc.publisher = PUBLISHER.to_string();

    // dc:date from datePublished
    if !graph.date_published.is_empty() {
        dc.dates.push(graph.date_published.clone());
    }

    // dc:type derived from typeOfData
    dc.resource_type = graph.general_data_type.clone();

    // dc:format — the file's MIME type (bitstream records only)
    if let Some(ref mime_type) = graph.mime_type {
        dc.formats.push(mime_type.clone());
    }

    // dc:relation — link to parent project
    dc.relations.push(graph.project_ark.clone());

    // dc:rights - license identifier and URI
    if let Some(ref label) = graph.license_label {
        dc.rights.push(label.clone());
    }
    if !graph.license_uri.is_empty() {
        dc.rights.push(graph.license_uri.clone());
    }

    dc
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
    fn identifier_is_pid() {
        let dc = record_to_dublin_core(&RecordGraph::build(&test_record(), ArkHost::Recorded));
        assert!(dc
            .identifiers
            .contains(&"https://ark.dasch.swiss/ark:/72163/1/0001/record-0001".to_string()));
    }

    #[test]
    fn title_prefers_english() {
        let dc = record_to_dublin_core(&RecordGraph::build(&test_record(), ArkHost::Recorded));
        assert_eq!(dc.titles, vec!["Survey Responses on Rural Land Use, 1920–1950"]);
    }

    #[test]
    fn creators_from_authorship() {
        let dc = record_to_dublin_core(&RecordGraph::build(&test_record(), ArkHost::Recorded));
        assert_eq!(dc.creators, vec!["Dr. Anna Müller", "Prof. Hans Bauer"]);
    }

    #[test]
    fn description_in_english() {
        let dc = record_to_dublin_core(&RecordGraph::build(&test_record(), ArkHost::Recorded));
        assert_eq!(dc.descriptions, vec!["A collection of survey responses."]);
    }

    #[test]
    fn publisher_is_dasch() {
        let dc = record_to_dublin_core(&RecordGraph::build(&test_record(), ArkHost::Recorded));
        assert_eq!(dc.publisher, "DaSCH");
    }

    #[test]
    fn date_is_date_published() {
        let dc = record_to_dublin_core(&RecordGraph::build(&test_record(), ArkHost::Recorded));
        assert!(dc.dates.contains(&"2024-02-01".to_string()));
    }

    #[test]
    fn resource_type_mapped_from_type_of_data() {
        let dc = record_to_dublin_core(&RecordGraph::build(&test_record(), ArkHost::Recorded));
        assert_eq!(dc.resource_type, "Text");
    }

    #[test]
    fn relation_links_to_parent_project() {
        let mut record = test_record();
        record.pid = Pid::new("https://ark.dasch.swiss", "0803", "lklK7rVuVOmpBZYWrF8o=gh");
        let dc = record_to_dublin_core(&RecordGraph::build(&record, ArkHost::Recorded));
        assert!(dc.relations.contains(&"https://ark.dasch.swiss/ark:/72163/1/0803".to_string()));
    }

    #[test]
    fn rights_contains_license_label_and_uri() {
        let dc = record_to_dublin_core(&RecordGraph::build(&test_record(), ArkHost::Recorded));
        assert!(dc
            .rights
            .contains(&"Creative Commons Attribution 4.0 International".to_string()));
        assert!(dc.rights.contains(&"https://creativecommons.org/licenses/by/4.0/".to_string()));
    }

    #[test]
    fn rights_are_empty_without_a_license() {
        let mut record = test_record();
        record.legal_info.license = RecordLicense::default();
        let dc = record_to_dublin_core(&RecordGraph::build(&record, ArkHost::Recorded));
        assert!(dc.rights.is_empty());
    }

    #[test]
    fn record_without_file_has_no_format() {
        let dc = record_to_dublin_core(&RecordGraph::build(&test_record(), ArkHost::Recorded));
        assert!(dc.formats.is_empty());
    }

    #[test]
    fn bitstream_format_is_mime_type() {
        let dc = record_to_dublin_core(&RecordGraph::build(&bitstream_record(), ArkHost::Recorded));
        assert_eq!(dc.formats, vec!["image/jp2"]);
    }

    #[test]
    fn bitstream_file_url_is_not_an_identifier() {
        let record = bitstream_record();
        let dc = record_to_dublin_core(&RecordGraph::build(&record, ArkHost::Recorded));

        assert_eq!(dc.identifiers, vec![record.pid.as_url()]);
        let file_url = record.file.as_ref().expect("bitstream record has a file").url.clone();
        assert!(!dc.identifiers.contains(&file_url));
        assert!(!dc.identifiers.iter().any(|i| i.contains("ingest.")));
        assert!(!dc.identifiers.iter().any(|i| i.contains("/dpe/records/")));
    }

    #[test]
    fn record_without_file_has_the_same_single_identifier() {
        let record = test_record();
        let dc = record_to_dublin_core(&RecordGraph::build(&record, ArkHost::Recorded));
        assert_eq!(dc.identifiers, vec![record.pid.as_url()]);
    }
}
