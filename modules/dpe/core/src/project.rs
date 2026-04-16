use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::cluster::ClusterRef;
use super::collection::CollectionRef;
use super::models::AuthorityFileReference;

/// Valid tab names for project detail pages.
pub const VALID_TABS: &[&str] = &["overview", "publications", "contributors"];

/// Returns true if the string is a valid project shortcode (alphanumeric only).
pub fn is_valid_shortcode(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric())
}

fn make_ref(url: String) -> AuthorityFileReference {
    AuthorityFileReference { type_: "URL".to_string(), url, text: None }
}

/// Parses the `"url"` JSON value — either a structured object (new format)
/// or a legacy string array — into primary and secondary references.
fn parse_url_value(value: Option<Value>) -> (Option<AuthorityFileReference>, Option<AuthorityFileReference>) {
    match value {
        Some(Value::Object(_)) => (serde_json::from_value::<AuthorityFileReference>(value.unwrap()).ok(), None),
        Some(Value::Array(arr)) => {
            let mut strings = arr.into_iter().filter_map(|v| v.as_str().map(str::to_string));
            (strings.next().map(make_ref), strings.next().map(make_ref))
        }
        _ => (None, None),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRaw {
    pub id: String,
    pub pid: String,
    pub name: String,
    pub shortcode: String,
    pub official_name: String,
    pub status: ProjectStatus,
    pub short_description: String,
    pub description: std::collections::HashMap<String, String>,
    pub start_date: String,
    pub end_date: String,
    /// Raw value — either a structured object or a legacy string array.
    #[serde(default)]
    pub url: Option<Value>,
    /// New-format secondary URL (absent in legacy files).
    pub secondary_url: Option<AuthorityFileReference>,
    pub how_to_cite: String,
    pub access_rights: AccessRights,
    pub legal_info: Vec<LegalInfo>,
    pub data_management_plan: Option<String>,
    pub data_publication_year: Option<String>,
    pub type_of_data: Option<Vec<String>>,
    pub data_language: Option<Vec<String>>,
    #[serde(default)]
    #[allow(dead_code)]
    pub clusters: Option<Vec<String>>,
    #[serde(default)]
    #[allow(dead_code)]
    pub collections: Option<Vec<String>>,
    #[serde(default)]
    pub records: Option<Vec<String>>,
    pub keywords: Vec<std::collections::HashMap<String, String>>,
    pub disciplines: Vec<Discipline>,
    pub temporal_coverage: Vec<TemporalCoverage>,
    pub spatial_coverage: Vec<AuthorityFileReference>,
    pub attributions: Vec<Attribution>,
    #[serde(rename = "abstract", default)]
    pub abstract_text: Option<std::collections::HashMap<String, String>>,
    pub contact_point: Option<Vec<String>>,
    #[serde(default)]
    pub publications: Option<Vec<Publication>>,
    pub funding: Funding,
    pub alternative_names: Option<Vec<std::collections::HashMap<String, String>>>,
    pub documentation_material: Option<Vec<String>>,
    #[serde(default)]
    pub provenance: Option<String>,
    pub additional_material: Option<Vec<String>>,
}

pub const ACCESS_RIGHTS_VALUES: &[&str] = &[
    "Full Open Access",
    "Open Access with Restrictions",
    "Embargoed Access",
    "Metadata only Access",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ProjectStatus {
    Ongoing,
    Finished,
}

impl ProjectStatus {
    pub fn is_ongoing(&self) -> bool {
        *self == ProjectStatus::Ongoing
    }

    pub fn is_finished(&self) -> bool {
        *self == ProjectStatus::Finished
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectStatus::Ongoing => "ongoing",
            ProjectStatus::Finished => "finished",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub pid: String,
    pub name: String,
    pub shortcode: String,
    pub official_name: String,
    pub status: ProjectStatus,
    pub short_description: String,
    pub description: std::collections::HashMap<String, String>,
    pub start_date: String,
    pub end_date: String,
    pub url: Option<AuthorityFileReference>,
    pub secondary_url: Option<AuthorityFileReference>,
    pub how_to_cite: String,
    pub access_rights: AccessRights,
    pub legal_info: Vec<LegalInfo>,
    pub data_management_plan: Option<String>,
    pub data_publication_year: Option<String>,
    pub type_of_data: Option<Vec<String>>,
    pub data_language: Option<Vec<String>>,
    pub clusters: Vec<ClusterRef>,
    pub collections: Vec<CollectionRef>,
    /// Raw collection IDs from JSON, used to resolve `collections` on demand.
    pub collection_ids: Vec<String>,
    pub records: Option<Vec<String>>,
    pub keywords: Vec<std::collections::HashMap<String, String>>,
    pub disciplines: Vec<Discipline>,
    pub temporal_coverage: Vec<TemporalCoverage>,
    pub spatial_coverage: Vec<AuthorityFileReference>,
    pub attributions: Vec<Attribution>,
    pub abstract_text: Option<std::collections::HashMap<String, String>>,
    pub contact_point: Option<Vec<String>>,
    pub publications: Option<Vec<Publication>>,
    pub funding: Funding,
    pub alternative_names: Option<Vec<std::collections::HashMap<String, String>>>,
    pub documentation_material: Option<Vec<String>>,
    pub provenance: Option<String>,
    pub additional_material: Option<Vec<String>>,
}

impl From<ProjectRaw> for Project {
    fn from(raw: ProjectRaw) -> Self {
        let (url, secondary_url_from_array) = parse_url_value(raw.url);
        // New-format files have `secondaryURL` as a separate key; legacy files encode
        // it as the second element of the `url` array.
        let secondary_url = raw.secondary_url.or(secondary_url_from_array);
        Project {
            id: raw.id,
            pid: raw.pid,
            name: raw.name,
            shortcode: raw.shortcode,
            official_name: raw.official_name,
            status: raw.status,
            short_description: raw.short_description,
            description: raw.description,
            start_date: raw.start_date,
            end_date: raw.end_date,
            url,
            secondary_url,
            how_to_cite: raw.how_to_cite,
            access_rights: raw.access_rights,
            legal_info: raw.legal_info,
            data_management_plan: raw.data_management_plan,
            data_publication_year: raw.data_publication_year,
            type_of_data: raw.type_of_data,
            data_language: raw.data_language,
            clusters: Vec::new(),
            collections: Vec::new(),
            collection_ids: raw.collections.unwrap_or_default(),
            records: raw.records,
            keywords: raw.keywords,
            disciplines: raw.disciplines,
            temporal_coverage: raw.temporal_coverage,
            spatial_coverage: raw.spatial_coverage,
            attributions: raw.attributions,
            abstract_text: raw.abstract_text,
            contact_point: raw.contact_point,
            publications: raw.publications,
            funding: raw.funding,
            alternative_names: raw.alternative_names,
            documentation_material: raw.documentation_material,
            provenance: raw.provenance,
            additional_material: raw.additional_material,
        }
    }
}

impl From<&Project> for ProjectRaw {
    fn from(p: &Project) -> Self {
        ProjectRaw {
            id: p.id.clone(),
            pid: p.pid.clone(),
            name: p.name.clone(),
            shortcode: p.shortcode.clone(),
            official_name: p.official_name.clone(),
            status: p.status.clone(),
            short_description: p.short_description.clone(),
            description: p.description.clone(),
            start_date: p.start_date.clone(),
            end_date: p.end_date.clone(),
            url: p.url.as_ref().and_then(|u| serde_json::to_value(u).ok()),
            secondary_url: p.secondary_url.clone(),
            how_to_cite: p.how_to_cite.clone(),
            access_rights: p.access_rights.clone(),
            legal_info: p.legal_info.clone(),
            data_management_plan: p.data_management_plan.clone(),
            data_publication_year: p.data_publication_year.clone(),
            type_of_data: p.type_of_data.clone(),
            data_language: p.data_language.clone(),
            clusters: None,
            collections: Some(p.collection_ids.clone()),
            records: p.records.clone(),
            keywords: p.keywords.clone(),
            disciplines: p.disciplines.clone(),
            temporal_coverage: p.temporal_coverage.clone(),
            spatial_coverage: p.spatial_coverage.clone(),
            attributions: p.attributions.clone(),
            abstract_text: p.abstract_text.clone(),
            contact_point: p.contact_point.clone(),
            publications: p.publications.clone(),
            funding: p.funding.clone(),
            alternative_names: p.alternative_names.clone(),
            documentation_material: p.documentation_material.clone(),
            provenance: p.provenance.clone(),
            additional_material: p.additional_material.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TemporalCoverage {
    Reference(AuthorityFileReference),
    Text(std::collections::HashMap<String, String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Discipline {
    Reference(AuthorityFileReference),
    Text(std::collections::HashMap<String, String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Funding {
    Grants(Vec<Grant>),
    Text(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessRights {
    #[serde(rename = "accessRights")]
    pub access_rights: AccessRightsType,
    #[serde(rename = "embargoDate", default)]
    pub embargo_date: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegalInfo {
    pub license: License,
    #[serde(rename = "copyrightHolder")]
    pub copyright_holder: String,
    pub authorship: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct License {
    #[serde(rename = "licenseIdentifier")]
    pub license_identifier: String,
    #[serde(rename = "licenseDate")]
    pub license_date: String,
    #[serde(rename = "licenseURI")]
    pub license_uri: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attribution {
    pub contributor: String,
    #[serde(rename = "contributorType")]
    pub contributor_type: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Publication {
    pub text: String,
    #[serde(default)]
    pub pid: Option<Pid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pid {
    pub url: String,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grant {
    pub funders: Vec<String>,
    #[serde(default)]
    pub number: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}
