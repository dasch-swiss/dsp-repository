use serde::{Deserialize, Serialize};

use super::project::Project;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthorityFileReference {
    #[serde(rename = "type")]
    pub type_: String,
    pub url: String,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Page {
    pub items: Vec<Project>,
    pub nr_pages: i32,
    pub total_items: i32,
}
