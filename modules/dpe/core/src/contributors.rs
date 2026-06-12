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

/// Heuristic for distinguishing organization IDs (e.g. `organization-001`,
/// `0803-organization-000`) from person IDs (e.g. `person-028`).
pub fn is_organization_id(id: &str) -> bool {
    id.starts_with("organization-") || id.contains("-organization-")
}

/// Lookup of persons and organizations by their internal ID.
///
/// Abstracted as a trait so consumers (e.g. OAI-PMH metadata transforms) can
/// be tested without the disk-backed caches.
pub trait ContributorLookup {
    fn person(&self, id: &str) -> Option<Person>;
    fn organization(&self, id: &str) -> Option<Organization>;
}

/// Production [`ContributorLookup`] backed by the in-process person and
/// organization caches.
#[cfg(not(target_arch = "wasm32"))]
pub struct CachedContributorLookup;

#[cfg(not(target_arch = "wasm32"))]
impl ContributorLookup for CachedContributorLookup {
    fn person(&self, id: &str) -> Option<Person> {
        load_person(id)
    }

    fn organization(&self, id: &str) -> Option<Organization> {
        load_organization(id)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_person(id: &str) -> Option<Person> {
    super::person_cache::all_persons().get(id).cloned()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_organization(id: &str) -> Option<Organization> {
    super::organization_cache::all_organizations().get(id).cloned()
}
