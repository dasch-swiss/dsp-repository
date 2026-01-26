use leptos::prelude::*;
use leptos_router::params::Params;
use serde::{Deserialize, Serialize};

use super::models::AuthorityFileReference;

#[derive(Clone, Debug, Serialize, Deserialize, Params, PartialEq, Default)]
pub struct ProjectQuery {
    pub ongoing: Option<bool>,
    pub finished: Option<bool>,
    pub search: Option<String>,
    pub page: Option<i32>,
}

impl ProjectQuery {
    pub fn ongoing(&self) -> bool {
        self.ongoing.unwrap_or(true)
    }

    pub fn finished(&self) -> bool {
        self.finished.unwrap_or(true)
    }

    pub fn search(&self) -> String {
        self.search.clone().unwrap_or_default()
    }

    pub fn page(&self) -> i32 {
        self.page.unwrap_or(1)
    }

    pub fn to_query_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(false) = self.ongoing {
            parts.push("ongoing=false".to_string());
        }
        if let Some(false) = self.finished {
            parts.push("finished=false".to_string());
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
    pub url: Vec<String>,
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
