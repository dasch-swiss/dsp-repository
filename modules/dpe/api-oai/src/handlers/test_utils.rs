#![cfg(test)]

use dpe_core::models::AuthorityFileReference;
use dpe_core::project::{
    AccessRights, AccessRightsType, Attribution, Discipline, Funding, Grant, LegalInfo, License, Project,
    ProjectStatus, TemporalCoverage,
};
use dpe_core::{ProjectRepository, Record, RecordRepository};

/// In-memory repository for testing.
pub struct InMemoryProjectRepository {
    projects: Vec<Project>,
}

impl InMemoryProjectRepository {
    pub fn new(projects: Vec<Project>) -> Self {
        Self { projects }
    }
}

impl ProjectRepository for InMemoryProjectRepository {
    fn get_all(&self) -> &[Project] {
        &self.projects
    }

    fn get_by_shortcode(&self, shortcode: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.shortcode == shortcode)
    }
}

/// Builds a minimal Project fixture based on the incunabula project (0803).
pub fn incunabula_project() -> Project {
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
        data_language: Some(vec!["de".to_string()]),
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
            map.insert(
                "en".to_string(),
                "An interdisciplinary research project on image sequences of Basel's early prints.".to_string(),
            );
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

/// Loads the first record from the 0803-records.json fixture.
pub fn first_0803_record() -> Record {
    let json = include_str!("../../../server/data/records/0803-records.json");
    let [record]: [Record; 1] = serde_json::from_str(json).expect("parse 0803-records.json");
    record
}

/// In-memory record repository for testing.
pub struct InMemoryRecordRepository {
    records: Vec<Record>,
}

impl InMemoryRecordRepository {
    pub fn new(records: Vec<Record>) -> Self {
        Self { records }
    }

    pub fn empty() -> Self {
        Self { records: vec![] }
    }
}

impl RecordRepository for InMemoryRecordRepository {
    fn get_all(&self) -> &[Record] {
        &self.records
    }

    fn get_by_id(&self, ark_suffix: &str) -> Option<&Record> {
        self.records.iter().find(|r| r.pid.ark_suffix() == ark_suffix)
    }
}

/// Strips the `<responseDate>` line from XML so golden comparisons are stable.
pub fn normalize(xml: &str) -> String {
    xml.lines()
        .filter(|l| !l.trim_start().starts_with("<responseDate>"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Loads a golden file, creating it if absent (first-run mode).
/// Compares and stores the normalized form (without responseDate).
pub fn golden(name: &str, actual: &str) -> String {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/oai/handlers/testdata/golden");
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

pub fn validate_against_schema(xml: &str) {
    let xsd_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/handlers/testdata/schemas/validate.xsd");

    let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
    std::io::Write::write_all(&mut tmp, xml.as_bytes()).expect("write temp file");

    let output = std::process::Command::new("xmllint")
        .arg("--noout")
        .arg("--schema")
        .arg(xsd_path)
        .arg(tmp.path())
        .output()
        .expect("xmllint must be available");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Schema validation failed:\n{}", stderr);
    }
}
