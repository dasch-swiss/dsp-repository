//! Handler for the OAI-PMH ListIdentifiers verb.

use app::domain::{ProjectRepository, RecordRepository};

use super::{build_error_response, build_list_request_params, validate_list_params, OaiParams};
use crate::oai::xml::OaiXmlBuilder;

/// Handles the ListIdentifiers verb.
pub fn handle_list_identifiers(
    params: &OaiParams,
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
) -> String {
    let (prefix, records) = match validate_list_params(params, repo, record_repo) {
        Ok(result) => result,
        Err(err) => return build_error_response(err),
    };

    let request_params = build_list_request_params(prefix, params);

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("ListIdentifiers", &request_params);

    builder.start_element("ListIdentifiers");
    for record in &records {
        builder.write_record_header(
            &record.header.identifier,
            &record.header.datestamp,
            &record.header.set_specs,
        );
    }
    builder.end_element("ListIdentifiers");

    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    use app::domain::{
        models::AuthorityFileReference,
        project::{
            AccessRights, AccessRightsType, Attribution, Discipline, Funding, Grant, LegalInfo,
            License, Project, ProjectStatus, TemporalCoverage,
        },
        Record, RecordLegalInfo, RecordLicense,
    };
    use std::collections::HashMap;

    struct InMemoryProjectRepository {
        projects: Vec<Project>,
    }

    impl InMemoryProjectRepository {
        fn new(projects: Vec<Project>) -> Self {
            Self { projects }
        }
    }

    impl ProjectRepository for InMemoryProjectRepository {
        fn get_all(&self) -> Vec<Project> {
            self.projects.clone()
        }
        fn get_by_shortcode(&self, shortcode: &str) -> Option<Project> {
            self.projects.iter().find(|p| p.shortcode == shortcode).cloned()
        }
    }

    struct InMemoryRecordRepository {
        records: Vec<Record>,
    }

    impl InMemoryRecordRepository {
        fn new(records: Vec<Record>) -> Self {
            Self { records }
        }
        fn empty() -> Self {
            Self { records: vec![] }
        }
    }

    impl RecordRepository for InMemoryRecordRepository {
        fn get_all(&self) -> Vec<Record> {
            self.records.clone()
        }
        fn get_by_id(&self, id: &str) -> Option<Record> {
            self.records.iter().find(|r| r.id == id).cloned()
        }
    }

    fn incunabula_project() -> Project {
        Project {
            id: "0803".to_string(),
            pid: "MISSING".to_string(),
            name: "Die Bilderfolgen der Basler Frühdrucke: Spätmittelalterliche Didaxe als Bild-Text-Lektüre".to_string(),
            shortcode: "0803".to_string(),
            official_name: "MISSING".to_string(),
            status: ProjectStatus::Finished,
            short_description: "An art-scientific monograph of the richly illustrated early prints in Basel.".to_string(),
            description: {
                let mut map = HashMap::new();
                map.insert("en".to_string(), "A description of early prints in Basel.".to_string());
                map
            },
            start_date: "2008-06-01".to_string(),
            end_date: "2012-08-31".to_string(),
            url: Some(AuthorityFileReference {
                type_: "URL".to_string(),
                url: "https://app.dasch.swiss/project/3ABR_2i8QYGSIDvmP9mlEw".to_string(),
                text: None,
            }),
            secondary_url: None,
            how_to_cite: "Incunabula (2012) DaSCH. ark.dasch.swiss/ark:/72163/1/0803".to_string(),
            access_rights: AccessRights {
                access_rights: AccessRightsType::FullOpenAccess,
                embargo_date: None,
            },
            legal_info: vec![LegalInfo {
                license: License {
                    license_identifier: "CC-BY-4.0".to_string(),
                    license_date: "2012-08-31".to_string(),
                    license_uri: "https://creativecommons.org/licenses/by/4.0/".to_string(),
                },
                copyright_holder: "MISSING".to_string(),
                authorship: vec!["MISSING".to_string()],
            }],
            data_management_plan: Some("not accessible".to_string()),
            data_publication_year: None,
            type_of_data: Some(vec!["Image".to_string()]),
            data_language: Some(vec![{
                let mut map = HashMap::new();
                map.insert("en".to_string(), "German".to_string());
                map
            }]),
            collections: None,
            records: None,
            keywords: vec![{
                let mut map = HashMap::new();
                map.insert("en".to_string(), "Letterpress Printing".to_string());
                map
            }],
            disciplines: vec![Discipline::Text({
                let mut map = HashMap::new();
                map.insert("en".to_string(), "10404 Visual arts and Art history".to_string());
                map
            })],
            temporal_coverage: vec![TemporalCoverage::Text({
                let mut map = HashMap::new();
                map.insert("en".to_string(), "Late Middle Ages".to_string());
                map
            })],
            spatial_coverage: vec![AuthorityFileReference {
                type_: "Geonames".to_string(),
                url: "https://www.geonames.org/2661604/basel.html".to_string(),
                text: Some("Basel".to_string()),
            }],
            attributions: vec![Attribution {
                contributor: "0803-person-000".to_string(),
                contributor_type: vec!["Applicant".to_string()],
            }],
            abstract_text: Some({
                let mut map = HashMap::new();
                map.insert("en".to_string(), "An interdisciplinary research project on image sequences of Basel's early prints.".to_string());
                map
            }),
            contact_point: None,
            publications: None,
            funding: Funding::Grants(vec![Grant {
                funders: vec!["0803-organization-000".to_string()],
                number: Some("120378".to_string()),
                name: Some("Project funding".to_string()),
                url: Some("https://data.snf.ch/grants/grant/120378".to_string()),
            }]),
            alternative_names: Some(vec![{
                let mut map = HashMap::new();
                map.insert("en".to_string(), "Incunabula".to_string());
                map
            }]),
            documentation_material: None,
            provenance: None,
            additional_material: None,
        }
    }

    fn test_record() -> Record {
        Record {
            id: "record-0001".to_string(),
            pid: "https://ark.dasch.swiss/ark:/72163/1/record-0001".to_string(),
            label: {
                let mut m = HashMap::new();
                m.insert("en".to_string(), "Survey Responses on Rural Land Use, 1920–1950".to_string());
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
                authorship: vec!["Dr. Anna Müller".to_string()],
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

    fn make_params(metadata_prefix: Option<&str>) -> OaiParams {
        OaiParams {
            verb: Some("ListIdentifiers".to_string()),
            identifier: None,
            metadata_prefix: metadata_prefix.map(str::to_string),
            from: None,
            until: None,
            set: None,
            resumption_token: None,
        }
    }

    use super::super::test_utils::{golden, normalize};

    // ---- error cases ----

    #[test]
    fn missing_metadata_prefix_returns_bad_argument() {
        let params = make_params(None);
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("metadataPrefix argument is required"), "got: {}", xml);
    }

    #[test]
    fn unsupported_metadata_prefix_returns_cannot_disseminate() {
        let params = make_params(Some("marc21"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"cannotDisseminateFormat\">"), "got: {}", xml);
    }

    #[test]
    fn resumption_token_returns_bad_resumption_token() {
        let mut params = make_params(Some("oai_dc"));
        params.resumption_token = Some("some-token".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"badResumptionToken\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_set_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Unknown".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn cluster_set_with_no_clusters_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:ProjectCluster".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn record_set_filter_returns_only_records() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let project_repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![test_record()]);
        let xml = handle_list_identifiers(&params, &project_repo, &record_repo);
        assert!(xml.contains("record-0001"), "should contain record identifier; got: {xml}");
        assert!(!xml.contains("oai:meta.dasch.swiss:ark:/72163/1/0803"), "should not contain project identifier; got: {xml}");
    }

    #[test]
    fn research_project_set_filter_excludes_records() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:ResearchProject".to_string());
        let project_repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![test_record()]);
        let xml = handle_list_identifiers(&params, &project_repo, &record_repo);
        assert!(xml.contains("0803"), "should contain project identifier; got: {xml}");
        assert!(!xml.contains("record-0001"), "should not contain record identifier; got: {xml}");
    }

    // ---- golden tests ----

    #[test]
    fn golden_oai_dc_response() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty());
        let expected = golden("list_identifiers_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_oai_datacite_response() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty());
        let expected = golden("list_identifiers_oai_datacite.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn list_identifiers_oai_dc_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty());
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_identifiers_oai_datacite_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty());
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }
}
