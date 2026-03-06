//! Handler for the OAI-PMH ListMetadataFormats verb.

use app::domain::ProjectRepository;

use super::{build_error_response, OaiParams};
use crate::oai::error::OaiError;
use crate::oai::metadata::parse_oai_identifier;
use crate::oai::xml::OaiXmlBuilder;

/// Handles the ListMetadataFormats verb.
pub fn handle_list_metadata_formats(params: &OaiParams, repo: &dyn ProjectRepository) -> String {
    // ListMetadataFormats accepts only identifier as optional argument
    if params.from.is_some()
        || params.until.is_some()
        || params.set.is_some()
        || params.resumption_token.is_some()
    {
        return build_error_response(OaiError::BadArgument(
            "Unexpected argument for ListMetadataFormats".to_string(),
        ));
    }

    // If identifier is provided, verify it exists
    if let Some(ref id) = params.identifier {
        if let Some(shortcode) = parse_oai_identifier(id) {
            if repo.get_by_shortcode(&shortcode).is_none() {
                return build_error_response(OaiError::IdDoesNotExist);
            }
        } else {
            return build_error_response(OaiError::IdDoesNotExist);
        }
    }

    let mut builder = OaiXmlBuilder::new();
    let mut request_params = vec![];
    if let Some(ref id) = params.identifier {
        request_params.push(("identifier", id.as_str()));
    }
    builder.write_request("ListMetadataFormats", &request_params);
    builder.write_list_metadata_formats();

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
            collections: None,
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

    fn make_params(identifier: Option<&str>) -> OaiParams {
        OaiParams {
            verb: Some("ListMetadataFormats".to_string()),
            identifier: identifier.map(str::to_string),
            metadata_prefix: None,
            from: None,
            until: None,
            set: None,
            resumption_token: None,
        }
    }

    fn normalize(xml: &str) -> String {
        xml.lines()
            .filter(|l| !l.trim_start().starts_with("<responseDate>"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn golden(name: &str, actual: &str) -> String {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/oai/handlers/testdata/golden");
        std::fs::create_dir_all(&dir).expect("create golden dir");
        let path = dir.join(name);
        let normalized = normalize(actual);
        if path.exists() {
            std::fs::read_to_string(&path).expect("read golden file")
        } else {
            std::fs::write(&path, &normalized).expect("write golden file");
            normalized
        }
    }

    // ---- error cases ----

    #[test]
    fn unexpected_argument_returns_bad_argument() {
        let mut params = make_params(None);
        params.from = Some("2020-01-01".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_metadata_formats(&params, &repo);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_identifier_returns_id_does_not_exist() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/9999"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_metadata_formats(&params, &repo);
        assert!(xml.contains("<error code=\"idDoesNotExist\">"), "got: {}", xml);
    }

    // ---- golden tests ----

    #[test]
    fn golden_response() {
        let params = make_params(None);
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_metadata_formats(&params, &repo);
        let expected = golden("list_metadata_formats.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn list_metadata_formats_response_is_valid_oai_pmh() {
        let params = make_params(None);
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_metadata_formats(&params, &repo);
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }
}
