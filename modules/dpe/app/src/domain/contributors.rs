use serde::{Deserialize, Serialize};

use super::organization::Organization;
use super::person::Person;
use super::project::Attribution;

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

#[cfg(feature = "ssr")]
async fn load_person(id: &str) -> Option<Person> {
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

#[cfg(feature = "ssr")]
async fn load_organization(id: &str) -> Option<Organization> {
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

#[leptos::server]
pub async fn get_contributors(
    attributions: Vec<Attribution>,
) -> Result<Vec<ResolvedContributor>, leptos::server_fn::error::ServerFnError> {
    let mut result = Vec::with_capacity(attributions.len());
    for attr in attributions {
        let roles = (!attr.contributor_type.is_empty()).then(|| attr.contributor_type.join(", "));
        let id = &attr.contributor;
        if id.contains("-organization-") {
            match load_organization(id).await {
                Some(org) => result.push(ResolvedContributor::Organization { org, roles }),
                None => result.push(ResolvedContributor::Unknown { id: id.clone(), roles }),
            }
        } else {
            match load_person(id).await {
                Some(person) => {
                    let mut affiliations = Vec::with_capacity(person.affiliations.len());
                    for aff_id in &person.affiliations {
                        if let Some(org) = load_organization(aff_id).await {
                            affiliations.push(org);
                        }
                    }
                    result.push(ResolvedContributor::Person { person, affiliations, roles });
                }
                None => result.push(ResolvedContributor::Unknown { id: id.clone(), roles }),
            }
        }
    }
    Ok(result)
}
