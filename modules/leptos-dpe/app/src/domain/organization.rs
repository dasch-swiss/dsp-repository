use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::models::AuthorityFileReference;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub pid: String,
    pub name: String,
    #[serde(rename = "sameAs", default)]
    pub same_as: Vec<AuthorityFileReference>,
    pub url: String,
    #[serde(default)]
    pub address: Option<Address>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(rename = "alternativeName", default)]
    pub alternative_name: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    #[serde(rename = "postalCode")]
    pub postal_code: String,
    pub locality: String,
    pub country: String,
    #[serde(default)]
    pub canton: Option<String>,
    #[serde(default)]
    pub additional: Option<String>,
}
