//! Handler for the OAI-PMH GetRecord verb.

use app::domain::{Project, ProjectRepository};

use super::{build_error_response, OaiParams, SUPPORTED_PREFIXES};
use crate::oai::error::OaiError;
use crate::oai::metadata::{parse_oai_identifier, ProjectOaiExt};
use crate::oai::xml::OaiXmlBuilder;

/// Handles the GetRecord verb.
pub fn handle_get_record(params: &OaiParams, repo: &dyn ProjectRepository) -> String {
    let result = require_identifier(params)
        .and_then(|id| require_metadata_prefix(params).map(|prefix| (id, prefix)))
        .and_then(|(id, prefix)| reject_unexpected_args(params).map(|_| (id, prefix)))
        .and_then(|(id, prefix)| resolve_project(id, repo).map(|project| (id, prefix, project)))
        .map(|(id, prefix, project)| build_response(id, prefix, &project));

    result.unwrap_or_else(build_error_response)
}

fn require_identifier(params: &OaiParams) -> Result<&str, OaiError> {
    params
        .identifier
        .as_deref()
        .ok_or_else(|| OaiError::BadArgument("identifier argument is required".to_string()))
}

fn require_metadata_prefix(params: &OaiParams) -> Result<&str, OaiError> {
    let prefix = params
        .metadata_prefix
        .as_deref()
        .ok_or_else(|| OaiError::BadArgument("metadataPrefix argument is required".to_string()))?;

    if !SUPPORTED_PREFIXES.contains(&prefix) {
        return Err(OaiError::CannotDisseminateFormat);
    }

    Ok(prefix)
}

fn reject_unexpected_args(params: &OaiParams) -> Result<(), OaiError> {
    if params.from.is_some()
        || params.until.is_some()
        || params.set.is_some()
        || params.resumption_token.is_some()
    {
        return Err(OaiError::BadArgument(
            "Unexpected argument for GetRecord".to_string(),
        ));
    }
    Ok(())
}

fn resolve_project(identifier: &str, repo: &dyn ProjectRepository) -> Result<Project, OaiError> {
    let shortcode = parse_oai_identifier(identifier).ok_or(OaiError::IdDoesNotExist)?;
    repo.get_by_shortcode(&shortcode).ok_or(OaiError::IdDoesNotExist)
}

fn build_response(identifier: &str, prefix: &str, project: &Project) -> String {
    let record = project.to_oai_record(prefix);

    let mut builder = OaiXmlBuilder::new();
    builder.write_request(
        "GetRecord",
        &[("identifier", identifier), ("metadataPrefix", prefix)],
    );
    builder.start_element("GetRecord");
    builder.write_record(&record);
    builder.end_element("GetRecord");
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    use app::domain::{
        project::{AccessRights, AccessRightsType, Attribution, Discipline, Funding, Project, ProjectStatus, TemporalCoverage},
        models::AuthorityFileReference,
    };

    /// In-memory repository for testing.
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

    /// Builds a minimal Project fixture based on the incunabula project (0803).
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
            legal_info: vec![app::domain::project::LegalInfo {
                license: app::domain::project::License {
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
            funding: Funding::Grants(vec![app::domain::project::Grant {
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

    fn make_params(identifier: Option<&str>, metadata_prefix: Option<&str>) -> OaiParams {
        OaiParams {
            verb: Some("GetRecord".to_string()),
            identifier: identifier.map(str::to_string),
            metadata_prefix: metadata_prefix.map(str::to_string),
            from: None,
            until: None,
            set: None,
            resumption_token: None,
        }
    }

    fn repo_with_incunabula() -> InMemoryProjectRepository {
        InMemoryProjectRepository::new(vec![incunabula_project()])
    }

    // ---- golden file helpers ----

    /// Strips the `<responseDate>` line from XML so golden comparisons are stable.
    fn normalize(xml: &str) -> String {
        xml.lines()
            .filter(|l| !l.trim_start().starts_with("<responseDate>"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Loads a golden file, creating it if absent (first-run mode).
    /// Compares and stores the normalized form (without responseDate).
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
    fn missing_identifier_returns_bad_argument() {
        let params = make_params(None, Some("oai_dc"));
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("identifier argument is required"), "got: {}", xml);
    }

    #[test]
    fn missing_metadata_prefix_returns_bad_argument() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/0803"), None);
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("metadataPrefix argument is required"), "got: {}", xml);
    }

    #[test]
    fn unsupported_metadata_prefix_returns_cannot_disseminate() {
        let params = make_params(Some("oai:meta.dasch.swiss:ark:/72163/1/0803"), Some("marc21"));
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        assert!(xml.contains("<error code=\"cannotDisseminateFormat\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_shortcode_returns_id_does_not_exist() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/9999"),
            Some("oai_dc"),
        );
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        assert!(xml.contains("<error code=\"idDoesNotExist\">"), "got: {}", xml);
    }

    #[test]
    fn unexpected_argument_returns_bad_argument() {
        let mut params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803"),
            Some("oai_dc"),
        );
        params.set = Some("entityType:ResearchProject".to_string());
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    // ---- golden tests ----

    #[test]
    fn golden_oai_dc_response() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803"),
            Some("oai_dc"),
        );
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        let expected = golden("get_record_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_oai_datacite_response() {
        let params = make_params(
            Some("oai:meta.dasch.swiss:ark:/72163/1/0803"),
            Some("oai_datacite"),
        );
        let repo = repo_with_incunabula();
        let xml = handle_get_record(&params, &repo);
        let expected = golden("get_record_oai_datacite.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }
}
