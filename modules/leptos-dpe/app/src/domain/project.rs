use leptos::prelude::*;
use leptos_router::params::Params;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::cluster::ClusterRef;
use super::collection::CollectionRef;
use super::models::AuthorityFileReference;

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

#[derive(Deserialize)]
pub(super) struct ProjectRaw {
    pub id: String,
    pub pid: String,
    pub name: String,
    pub shortcode: String,
    #[serde(rename = "officialName")]
    pub official_name: String,
    pub status: ProjectStatus,
    #[serde(rename = "shortDescription")]
    pub short_description: String,
    pub description: std::collections::HashMap<String, String>,
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: String,
    /// Raw value — either a structured object or a legacy string array.
    #[serde(default)]
    pub url: Option<Value>,
    /// New-format secondary URL (absent in legacy files).
    #[serde(rename = "secondaryURL", default)]
    pub secondary_url: Option<AuthorityFileReference>,
    #[serde(rename = "howToCite")]
    pub how_to_cite: String,
    #[serde(rename = "accessRights")]
    pub access_rights: AccessRights,
    #[serde(rename = "legalInfo")]
    pub legal_info: Vec<LegalInfo>,
    #[serde(rename = "dataManagementPlan", default)]
    pub data_management_plan: Option<String>,
    #[serde(rename = "dataPublicationYear", default)]
    pub data_publication_year: Option<String>,
    #[serde(rename = "typeOfData", default)]
    pub type_of_data: Option<Vec<String>>,
    #[serde(rename = "dataLanguage", default)]
    pub data_language: Option<Vec<std::collections::HashMap<String, String>>>,
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
    #[serde(rename = "temporalCoverage")]
    pub temporal_coverage: Vec<TemporalCoverage>,
    #[serde(rename = "spatialCoverage")]
    pub spatial_coverage: Vec<AuthorityFileReference>,
    pub attributions: Vec<Attribution>,
    #[serde(rename = "abstract", default)]
    pub abstract_text: Option<std::collections::HashMap<String, String>>,
    #[serde(rename = "contactPoint", default)]
    pub contact_point: Option<Vec<String>>,
    #[serde(default)]
    pub publications: Option<Vec<Publication>>,
    pub funding: Funding,
    #[serde(rename = "alternativeNames", default)]
    pub alternative_names: Option<Vec<std::collections::HashMap<String, String>>>,
    #[serde(rename = "documentationMaterial", default)]
    pub documentation_material: Option<Vec<String>>,
    #[serde(default)]
    pub provenance: Option<String>,
    #[serde(rename = "additionalMaterial", default)]
    pub additional_material: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Params, PartialEq, Default)]
pub struct ProjectQuery {
    pub ongoing: Option<bool>,
    pub finished: Option<bool>,
    pub search: Option<String>,
    pub page: Option<i32>,
    pub type_of_data: Option<String>,
    pub data_language: Option<String>,
    pub access_rights: Option<String>,
    pub dialog: Option<bool>,
}

pub const ACCESS_RIGHTS_VALUES: &[&str] = &[
    "Full Open Access",
    "Open Access with Restrictions",
    "Embargoed Access",
    "Metadata only Access",
];

impl ProjectQuery {
    pub fn ongoing(&self) -> bool {
        self.ongoing.unwrap_or(false)
    }

    pub fn finished(&self) -> bool {
        self.finished.unwrap_or(false)
    }

    pub fn search(&self) -> String {
        self.search.clone().unwrap_or_default()
    }

    pub fn page(&self) -> i32 {
        self.page.unwrap_or(1)
    }

    pub fn type_of_data(&self) -> Vec<String> {
        self.type_of_data
            .as_deref()
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default()
    }

    pub fn data_language(&self) -> Vec<String> {
        self.data_language
            .as_deref()
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default()
    }

    pub fn access_rights(&self) -> Vec<String> {
        self.access_rights
            .as_deref()
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default()
    }

    pub fn with_page(self, page: i32) -> Self {
        Self { page: Some(page), ..self }
    }

    /// Return a new `ProjectQuery` with the given status param toggled, page reset to 1.
    pub fn with_status_toggled(&self, param: &str) -> Self {
        let ongoing = self.ongoing();
        let finished = self.finished();
        Self {
            ongoing: Some(if param == "ongoing" { !ongoing } else { ongoing }),
            finished: Some(if param == "finished" { !finished } else { finished }),
            search: self.search.clone(),
            page: Some(1),
            type_of_data: self.type_of_data.clone(),
            data_language: self.data_language.clone(),
            access_rights: self.access_rights.clone(),
            dialog: self.dialog,
        }
    }

    /// Build access rights filter items: `(label, checked, href)` for each access rights value.
    pub fn access_rights_filter_items(&self) -> Vec<(String, bool, String)> {
        ACCESS_RIGHTS_VALUES
            .iter()
            .map(|&v| {
                let checked = self.access_rights().contains(&v.to_string());
                let href =
                    format!("/projects{}", self.with_access_rights_toggled(v).to_query_string());
                (v.to_string(), checked, href)
            })
            .collect()
    }

    /// Build status filter items: `(label, checked, href)` for "Ongoing" and "Finished".
    pub fn status_filter_items(&self) -> Vec<(String, bool, String)> {
        [("ongoing", "Ongoing", self.ongoing()), ("finished", "Finished", self.finished())]
            .iter()
            .map(|(param, label, checked)| {
                let href = format!("/projects{}", self.with_status_toggled(param).to_query_string());
                (label.to_string(), *checked, href)
            })
            .collect()
    }

    /// Return a new `ProjectQuery` with `value` toggled in/out of `type_of_data`, page reset to 1.
    pub fn with_type_of_data_toggled(&self, value: &str) -> Self {
        let mut selected = self.type_of_data();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        Self {
            ongoing: self.ongoing,
            finished: self.finished,
            search: self.search.clone(),
            page: Some(1),
            type_of_data: if selected.is_empty() { None } else { Some(selected.join(",")) },
            data_language: self.data_language.clone(),
            access_rights: self.access_rights.clone(),
            dialog: self.dialog,
        }
    }

    /// Return a new `ProjectQuery` with `value` toggled in/out of `data_language`, page reset to 1.
    pub fn with_data_language_toggled(&self, value: &str) -> Self {
        let mut selected = self.data_language();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        Self {
            ongoing: self.ongoing,
            finished: self.finished,
            search: self.search.clone(),
            page: Some(1),
            type_of_data: self.type_of_data.clone(),
            data_language: if selected.is_empty() { None } else { Some(selected.join(",")) },
            access_rights: self.access_rights.clone(),
            dialog: self.dialog,
        }
    }

    /// Return a new `ProjectQuery` with `value` toggled in/out of `access_rights`, page reset to 1.
    pub fn with_access_rights_toggled(&self, value: &str) -> Self {
        let mut selected = self.access_rights();
        if selected.contains(&value.to_string()) {
            selected.retain(|v| v != value);
        } else {
            selected.push(value.to_string());
        }
        Self {
            ongoing: self.ongoing,
            finished: self.finished,
            search: self.search.clone(),
            page: Some(1),
            type_of_data: self.type_of_data.clone(),
            data_language: self.data_language.clone(),
            access_rights: if selected.is_empty() { None } else { Some(selected.join(",")) },
            dialog: self.dialog,
        }
    }

    /// Return a new `ProjectQuery` with `dialog` set to `open`, preserving all other fields.
    pub fn with_dialog(self, open: bool) -> Self {
        Self { dialog: if open { Some(true) } else { None }, ..self }
    }

    pub fn to_query_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(true) = self.ongoing {
            parts.push("ongoing=true".to_string());
        }
        if let Some(true) = self.finished {
            parts.push("finished=true".to_string());
        }
        if let Some(ref search) = self.search {
            if !search.is_empty() {
                parts.push(format!("search={}", urlencoding::encode(search)));
            }
        }
        if let Some(page) = self.page {
            if page > 1 {
                parts.push(format!("page={}", page));
            }
        }
        if let Some(ref type_of_data) = self.type_of_data {
            if !type_of_data.is_empty() {
                parts.push(format!("type_of_data={}", type_of_data));
            }
        }
        if let Some(ref data_language) = self.data_language {
            if !data_language.is_empty() {
                parts.push(format!("data_language={}", data_language));
            }
        }
        if let Some(ref access_rights) = self.access_rights {
            if !access_rights.is_empty() {
                parts.push(format!("access_rights={}", access_rights));
            }
        }
        if let Some(true) = self.dialog {
            parts.push("dialog=true".to_string());
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("?{}", parts.join("&"))
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ProjectStatus {
    Ongoing,
    Finished,
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
    pub data_language: Option<Vec<std::collections::HashMap<String, String>>>,
    pub clusters: Vec<ClusterRef>,
    pub collections: Vec<CollectionRef>,
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
