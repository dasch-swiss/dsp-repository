use serde::{Deserialize, Serialize};
use shared_metadata::{ContributorLookup, Organization, Person};

use super::corpus::Corpus;

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

/// Production [`ContributorLookup`] backed by a corpus's person and
/// organization caches.
pub struct CachedContributorLookup {
    pub corpus: &'static Corpus,
}

impl ContributorLookup for CachedContributorLookup {
    fn person(&self, id: &str) -> Option<Person> {
        self.corpus.load_person(id)
    }

    fn organization(&self, id: &str) -> Option<Organization> {
        self.corpus.load_organization(id)
    }
}

impl Corpus {
    pub fn load_person(&'static self, id: &str) -> Option<Person> {
        self.all_persons().get(id).cloned()
    }

    pub fn load_organization(&'static self, id: &str) -> Option<Organization> {
        self.all_organizations().get(id).cloned()
    }
}
