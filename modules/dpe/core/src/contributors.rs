use serde::{Deserialize, Serialize};
use shared_metadata::{ContributorLookup, Organization, Person};

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

/// Production [`ContributorLookup`] backed by the in-process person and
/// organization caches.
pub struct CachedContributorLookup;

impl ContributorLookup for CachedContributorLookup {
    fn person(&self, id: &str) -> Option<Person> {
        load_person(id)
    }

    fn organization(&self, id: &str) -> Option<Organization> {
        load_organization(id)
    }
}

pub fn load_person(id: &str) -> Option<Person> {
    super::person_cache::all_persons().get(id).cloned()
}

pub fn load_organization(id: &str) -> Option<Organization> {
    super::organization_cache::all_organizations().get(id).cloned()
}
