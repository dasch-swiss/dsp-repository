use serde::{Deserialize, Serialize};

use super::organization::Organization;
use super::person::Person;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ResolvedContributor {
    Person {
        person: Person,
        /// Resolved affiliation organizations (same order as `person.affiliations`)
        affiliations: Vec<Organization>,
        roles: Option<String>,
    },
    Organization {
        org: Organization,
        roles: Option<String>,
    },
    Unknown {
        id: String,
        roles: Option<String>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_person(id: &str) -> Option<Person> {
    use std::fs;
    use std::path::PathBuf;

    use super::utils::get_data_dir;

    let persons_dir = PathBuf::from(get_data_dir()).join("persons");
    let entries = fs::read_dir(persons_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name()?.to_str()?;
            if name.starts_with(id) && name.ends_with(".json") {
                let json = fs::read_to_string(&path).ok()?;
                return serde_json::from_str(&json).ok();
            }
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_organization(id: &str) -> Option<Organization> {
    use std::fs;
    use std::path::PathBuf;

    use super::utils::get_data_dir;

    let orgs_dir = PathBuf::from(get_data_dir()).join("organizations");
    let entries = fs::read_dir(orgs_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name()?.to_str()?;
            if name.starts_with(id) && name.ends_with(".json") {
                let json = fs::read_to_string(&path).ok()?;
                return serde_json::from_str(&json).ok();
            }
        }
    }
    None
}
