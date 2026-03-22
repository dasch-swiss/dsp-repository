//! Handler for the OAI-PMH ListIdentifiers verb.

use app::domain::ProjectRepository;

use super::{build_error_response, build_list_request_params, validate_list_params, OaiParams};
use crate::oai::xml::OaiXmlBuilder;

/// Handles the ListIdentifiers verb.
pub fn handle_list_identifiers(params: &OaiParams, repo: &dyn ProjectRepository) -> String {
    let (prefix, records) = match validate_list_params(params, repo) {
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
    };

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
                let mut map = std::collections::HashMap::new();
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
                let mut map = std::collections::HashMap::new();
                map.insert("en".to_string(), "German".to_string());
                map
            }]),
            clusters: vec![],
            collections: vec![],
            collection_ids: vec![],
            records: None,
            keywords: vec![{
                let mut map = std::collections::HashMap::new();
                map.insert("en".to_string(), "Letterpress Printing".to_string());
                map
            }],
            disciplines: vec![Discipline::Text({
                let mut map = std::collections::HashMap::new();
                map.insert("en".to_string(), "10404 Visual arts and Art history".to_string());
                map
            })],
            temporal_coverage: vec![TemporalCoverage::Text({
                let mut map = std::collections::HashMap::new();
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
                let mut map = std::collections::HashMap::new();
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
                let mut map = std::collections::HashMap::new();
                map.insert("en".to_string(), "Incunabula".to_string());
                map
            }]),
            documentation_material: None,
            provenance: None,
            additional_material: None,
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
        let xml = handle_list_identifiers(&params, &repo);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("metadataPrefix argument is required"), "got: {}", xml);
    }

    #[test]
    fn unsupported_metadata_prefix_returns_cannot_disseminate() {
        let params = make_params(Some("marc21"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo);
        assert!(xml.contains("<error code=\"cannotDisseminateFormat\">"), "got: {}", xml);
    }

    #[test]
    fn resumption_token_returns_bad_resumption_token() {
        let mut params = make_params(Some("oai_dc"));
        params.resumption_token = Some("some-token".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo);
        assert!(xml.contains("<error code=\"badResumptionToken\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_set_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Unknown".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo);
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn cluster_set_with_no_clusters_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:ProjectCluster".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo);
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    // ---- golden tests ----

    #[test]
    fn golden_oai_dc_response() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo);
        let expected = golden("list_identifiers_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_oai_datacite_response() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo);
        let expected = golden("list_identifiers_oai_datacite.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn list_identifiers_oai_dc_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo);
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_identifiers_oai_datacite_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_identifiers(&params, &repo);
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }
}
