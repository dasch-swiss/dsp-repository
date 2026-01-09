use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub pid: String,
    pub name: String,
    pub shortcode: String,
    pub official_name: String,
    pub status: ProjectStatus,
    pub short_description: String,
    pub description: HashMap<String, String>,
    pub start_date: String,
    pub end_date: String,
    pub url: Vec<String>,
    pub how_to_cite: String,
    pub access_rights: AccessRights,
    pub legal_info: Vec<LegalInfo>,
    #[serde(default = "default_data_management_plan")]
    pub data_management_plan: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_publication_year: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_of_data: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_language: Option<Vec<HashMap<String, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collections: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records: Option<Vec<String>>,
    pub keywords: Vec<HashMap<String, String>>,
    pub disciplines: Vec<DisciplineItem>,
    pub temporal_coverage: Vec<TemporalCoverageItem>,
    pub spatial_coverage: Vec<AuthorityFileReference>,
    pub attributions: Vec<Attribution>,
    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_point: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publications: Option<Vec<Publication>>,
    #[serde(default = "default_funding")]
    pub funding: FundingType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative_names: Option<Vec<HashMap<String, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_material: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_material: Option<Vec<String>>,
}

fn default_data_management_plan() -> String {
    "not accessible".to_string()
}

fn default_funding() -> FundingType {
    FundingType::String("No funding".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FundingType {
    Grants(Vec<Grant>),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectStatus {
    Ongoing,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessRights {
    pub access_rights: AccessRightsType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embargo_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessRightsType {
    #[serde(rename = "Full Open Access")]
    FullOpenAccess,
    #[serde(rename = "Open Access with Restrictions")]
    OpenAccessWithRestrictions,
    #[serde(rename = "Embargoed Access")]
    EmbargoedAccess,
    #[serde(rename = "Metadata only Access")]
    MetadataOnlyAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegalInfo {
    pub license: License,
    pub copyright_holder: String,
    pub authorship: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct License {
    pub license_identifier: String,
    pub license_date: String,
    #[serde(rename = "licenseURI")]
    pub license_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attribution {
    pub contributor: String,
    pub contributor_type: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DisciplineItem {
    LocalizedText(HashMap<String, String>),
    Reference(AuthorityFileReference),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TemporalCoverageItem {
    LocalizedText(HashMap<String, String>),
    Reference(AuthorityFileReference),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityFileReference {
    #[serde(rename = "type")]
    pub reference_type: AuthorityFileReferenceType,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorityFileReferenceType {
    Geonames,
    Pleiades,
    Skos,
    Periodo,
    Chronontology,
    #[serde(rename = "GND")]
    Gnd,
    #[serde(rename = "VIAF")]
    Viaf,
    Grid,
    #[serde(rename = "ORCID")]
    Orcid,
    #[serde(rename = "Creative Commons")]
    CreativeCommons,
    #[serde(rename = "COAR")]
    Coar,
    #[serde(rename = "ROR")]
    Ror,
    #[serde(rename = "DOI")]
    Doi,
    #[serde(rename = "ARK")]
    Ark,
    #[serde(rename = "URL")]
    Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publication {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<Pid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pid {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grant {
    pub funders: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_project_json() {
        let json_str = include_str!("../../../../data/future/projects/0101_religious-speech.json");
        let project: Result<Project, _> = serde_json::from_str(json_str).map_err(|_| ());

        let p2: Project = project.clone().unwrap();

        // println!("{:#?}", project);
        let json_pretty = serde_json::to_string_pretty(&p2).unwrap();
        println!("{}", json_pretty);

        assert!(project.is_ok(), "Failed to parse project JSON: {:?}", project.err());

        let project = project.unwrap();
        assert_eq!(project.id, "0101");
        assert_eq!(project.shortcode, "0101");
        assert_eq!(project.status, ProjectStatus::Finished);
        assert_eq!(project.access_rights.access_rights, AccessRightsType::FullOpenAccess);
        assert!(!project.keywords.is_empty());
        assert!(!project.disciplines.is_empty());
        assert!(!project.attributions.is_empty());
    }

    #[test]
    fn test_access_rights_types() {
        let json = r#"{"accessRights": "Full Open Access"}"#;
        let ar: AccessRights = serde_json::from_str(json).unwrap();
        assert!(matches!(ar.access_rights, AccessRightsType::FullOpenAccess));
    }

    #[test]
    fn test_funding_as_string() {
        let json = r#""No funding""#;
        let funding: FundingType = serde_json::from_str(json).unwrap();
        assert!(matches!(funding, FundingType::String(_)));
    }

    #[test]
    fn test_funding_as_grants() {
        let json = r#"[{"funders": ["org-1"], "number": "123"}]"#;
        let funding: FundingType = serde_json::from_str(json).unwrap();
        assert!(matches!(funding, FundingType::Grants(_)));
    }
}
