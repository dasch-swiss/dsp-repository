//! Rules for reading contributor IDs and looking up the people and
//! organizations they reference, expressed entirely in terms of
//! `shared_metadata::Person` and `Organization`.

use crate::{Organization, Person};

/// Heuristic for distinguishing organization IDs (e.g. `organization-001`)
/// from person IDs (e.g. `person-028`).
pub fn is_organization_id(id: &str) -> bool {
    id.starts_with("organization-")
}

/// Lookup of persons and organizations by their internal ID.
///
/// Abstracted as a trait so consumers (e.g. OAI-PMH metadata transforms) can
/// be tested without the disk-backed caches.
pub trait ContributorLookup {
    fn person(&self, id: &str) -> Option<Person>;
    fn organization(&self, id: &str) -> Option<Organization>;
}

#[cfg(test)]
mod tests {
    use super::is_organization_id;

    #[test]
    fn organization_ids_are_recognised() {
        assert!(is_organization_id("organization-000"));
        assert!(is_organization_id("organization-142"));
    }

    #[test]
    fn person_ids_are_not_organizations() {
        assert!(!is_organization_id("person-028"));
    }
}
