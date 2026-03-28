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
    super::person_cache::all_persons().get(id).cloned()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_organization(id: &str) -> Option<Organization> {
    super::organization_cache::all_organizations().get(id).cloned()
}
