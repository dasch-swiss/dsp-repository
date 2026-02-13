use serde::{Deserialize, Serialize};

use super::models::AuthorityFileReference;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    pub pid: String,
    #[serde(rename = "givenNames")]
    pub given_names: Vec<String>,
    #[serde(rename = "familyNames")]
    pub family_names: Vec<String>,
    #[serde(rename = "jobTitles")]
    pub job_titles: Vec<String>,
    #[serde(default)]
    pub affiliations: Vec<String>,
    #[serde(rename = "sameAs", default)]
    pub same_as: Vec<AuthorityFileReference>,
    #[serde(default)]
    pub email: Option<String>,
}
