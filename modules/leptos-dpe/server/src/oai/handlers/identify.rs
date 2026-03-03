//! Handler for the OAI-PMH Identify verb.

use app::domain::ProjectRepository;

use super::{build_error_response, OaiParams};
use crate::oai::error::OaiError;
use crate::oai::xml::{OaiXmlBuilder, EARLIEST_DATESTAMP};

/// Handles the Identify verb.
pub fn handle_identify(params: &OaiParams, repo: &dyn ProjectRepository) -> String {
    // Identify does not accept any parameters except verb
    if params.identifier.is_some()
        || params.metadata_prefix.is_some()
        || params.from.is_some()
        || params.until.is_some()
        || params.set.is_some()
        || params.resumption_token.is_some()
    {
        return build_error_response(OaiError::BadArgument(
            "Identify does not accept any arguments".to_string(),
        ));
    }

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("Identify", &[]);

    let earliest = get_earliest_datestamp(repo);
    builder.write_identify(&earliest);

    builder.finish()
}

fn get_earliest_datestamp(repo: &dyn ProjectRepository) -> String {
    let projects = repo.get_all();
    projects
        .iter()
        .filter_map(|p| {
            if p.start_date != "MISSING" && !p.start_date.is_empty() {
                Some(p.start_date.as_str())
            } else {
                None
            }
        })
        .min()
        .unwrap_or(EARLIEST_DATESTAMP)
        .to_string()
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

    fn make_params() -> OaiParams {
        OaiParams {
            verb: Some("Identify".to_string()),
            identifier: None,
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
        let mut params = make_params();
        params.set = Some("entityType:ResearchProject".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_identify(&params, &repo);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("Identify does not accept any arguments"), "got: {}", xml);
    }

    // ---- golden tests ----

    #[test]
    fn golden_identify_response() {
        let params = make_params();
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_identify(&params, &repo);
        let expected = golden("identify.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_identify_empty_repo_response() {
        let params = make_params();
        let repo = InMemoryProjectRepository::new(vec![]);
        let xml = handle_identify(&params, &repo);
        let expected = golden("identify_empty_repo.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }
}
