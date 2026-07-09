//! Resolution of internal contributor IDs (e.g. `person-028`,
//! `organization-001`) to person and organization details for
//! metadata output.

use dpe_core::{is_organization_id, ContributorLookup, Person};

use super::types::DataCiteNameIdentifier;

/// Name details for a creator, contributor, or funder resolved from an
/// internal ID.
pub struct ResolvedAgent {
    /// Display name: `Family, Given` for persons, the organization name for
    /// organizations, or the raw ID if unresolvable.
    pub name: String,
    /// DataCite nameType: `Personal` or `Organizational`.
    pub name_type: &'static str,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub name_identifiers: Vec<DataCiteNameIdentifier>,
    /// Resolved affiliation organization names.
    pub affiliations: Vec<String>,
}

/// Resolves a contributor ID to name details. Unresolvable IDs fall back to
/// the raw ID so no attribution is lost.
pub fn resolve_agent(id: &str, lookup: &dyn ContributorLookup) -> ResolvedAgent {
    if is_organization_id(id) {
        if let Some(org) = lookup.organization(id) {
            return ResolvedAgent {
                name: org.name,
                name_type: "Organizational",
                given_name: None,
                family_name: None,
                name_identifiers: vec![],
                affiliations: vec![],
            };
        }
    } else if let Some(person) = lookup.person(id) {
        return person_to_agent(&person, lookup);
    }

    ResolvedAgent {
        name: id.to_string(),
        name_type: if is_organization_id(id) {
            "Organizational"
        } else {
            "Personal"
        },
        given_name: None,
        family_name: None,
        name_identifiers: vec![],
        affiliations: vec![],
    }
}

fn person_to_agent(person: &Person, lookup: &dyn ContributorLookup) -> ResolvedAgent {
    let given_name = non_empty(person.given_names.join(" "));
    let family_name = non_empty(person.family_names.join(" "));

    let name = match (&family_name, &given_name) {
        (Some(family), Some(given)) => format!("{}, {}", family, given),
        (Some(family), None) => family.clone(),
        (None, Some(given)) => given.clone(),
        (None, None) => person.id.clone(),
    };

    let name_identifiers = person
        .same_as
        .iter()
        .filter_map(|r| {
            let scheme_uri = match r.type_.as_str() {
                "ORCID" => "https://orcid.org",
                "GND" => "https://d-nb.info/gnd",
                _ => return None,
            };
            Some(DataCiteNameIdentifier {
                identifier: r.url.clone(),
                scheme: r.type_.clone(),
                scheme_uri: Some(scheme_uri.to_string()),
            })
        })
        .collect();

    let affiliations = person
        .affiliations
        .iter()
        .filter_map(|aff_id| lookup.organization(aff_id).map(|org| org.name))
        .collect();

    ResolvedAgent {
        name,
        name_type: "Personal",
        given_name,
        family_name,
        name_identifiers,
        affiliations,
    }
}

fn non_empty(s: String) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use dpe_core::models::AuthorityFileReference;
    use dpe_core::{Organization, Person};

    use super::*;
    use crate::handlers::test_utils::InMemoryContributorLookup;

    fn person(id: &str) -> Person {
        Person {
            id: id.to_string(),
            given_names: vec!["Anna".to_string()],
            family_names: vec!["Müller".to_string()],
            job_titles: vec![],
            affiliations: vec![],
            same_as: vec![],
            email: None,
        }
    }

    fn organization(id: &str, name: &str) -> Organization {
        Organization {
            id: id.to_string(),
            name: name.to_string(),
            same_as: vec![],
            url: String::new(),
            address: None,
            email: None,
            alternative_name: None,
        }
    }

    #[test]
    fn resolves_person_to_family_comma_given() {
        let lookup = InMemoryContributorLookup::empty().with_person(person("person-001"));
        let agent = resolve_agent("person-001", &lookup);
        assert_eq!(agent.name, "Müller, Anna");
        assert_eq!(agent.name_type, "Personal");
        assert_eq!(agent.given_name.as_deref(), Some("Anna"));
        assert_eq!(agent.family_name.as_deref(), Some("Müller"));
    }

    #[test]
    fn joins_multiple_given_and_family_names_with_spaces() {
        let mut p = person("person-001");
        p.given_names = vec!["Anna".to_string(), "Maria".to_string()];
        p.family_names = vec!["Roca".to_string(), "Escoda".to_string()];
        let lookup = InMemoryContributorLookup::empty().with_person(p);
        let agent = resolve_agent("person-001", &lookup);
        assert_eq!(agent.name, "Roca Escoda, Anna Maria");
        assert_eq!(agent.given_name.as_deref(), Some("Anna Maria"));
        assert_eq!(agent.family_name.as_deref(), Some("Roca Escoda"));
    }

    #[test]
    fn maps_orcid_and_gnd_but_not_url_to_name_identifiers() {
        let mut p = person("person-001");
        p.same_as = vec![
            AuthorityFileReference {
                type_: "ORCID".to_string(),
                url: "https://orcid.org/0000-0002-1825-0097".to_string(),
                text: None,
            },
            AuthorityFileReference {
                type_: "GND".to_string(),
                url: "https://d-nb.info/gnd/118540238".to_string(),
                text: None,
            },
            AuthorityFileReference {
                type_: "URL".to_string(),
                url: "https://example.com/anna".to_string(),
                text: None,
            },
        ];
        let lookup = InMemoryContributorLookup::empty().with_person(p);
        let agent = resolve_agent("person-001", &lookup);
        assert_eq!(agent.name_identifiers.len(), 2);
        assert_eq!(agent.name_identifiers[0].scheme, "ORCID");
        assert_eq!(agent.name_identifiers[0].identifier, "https://orcid.org/0000-0002-1825-0097");
        assert_eq!(agent.name_identifiers[0].scheme_uri.as_deref(), Some("https://orcid.org"));
        assert_eq!(agent.name_identifiers[1].scheme, "GND");
        assert_eq!(agent.name_identifiers[1].scheme_uri.as_deref(), Some("https://d-nb.info/gnd"));
    }

    #[test]
    fn resolves_affiliations_to_organization_names() {
        let mut p = person("person-001");
        p.affiliations = vec!["organization-001".to_string(), "organization-999".to_string()];
        let lookup = InMemoryContributorLookup::empty()
            .with_person(p)
            .with_organization(organization("organization-001", "Université de Lausanne"));
        let agent = resolve_agent("person-001", &lookup);
        // Unresolvable affiliation IDs are skipped, not emitted as raw IDs.
        assert_eq!(agent.affiliations, vec!["Université de Lausanne"]);
    }

    #[test]
    fn resolves_organization_contributor() {
        let lookup =
            InMemoryContributorLookup::empty().with_organization(organization("organization-000", "Universität Basel"));
        let agent = resolve_agent("organization-000", &lookup);
        assert_eq!(agent.name, "Universität Basel");
        assert_eq!(agent.name_type, "Organizational");
        assert_eq!(agent.given_name, None);
        assert_eq!(agent.family_name, None);
    }

    #[test]
    fn unresolvable_person_id_falls_back_to_raw_id() {
        let lookup = InMemoryContributorLookup::empty();
        let agent = resolve_agent("person-028", &lookup);
        assert_eq!(agent.name, "person-028");
        assert_eq!(agent.name_type, "Personal");
        assert!(agent.name_identifiers.is_empty());
        assert!(agent.affiliations.is_empty());
    }

    #[test]
    fn unresolvable_organization_id_falls_back_to_raw_id() {
        let lookup = InMemoryContributorLookup::empty();
        let agent = resolve_agent("organization-000", &lookup);
        assert_eq!(agent.name, "organization-000");
        assert_eq!(agent.name_type, "Organizational");
    }

    #[test]
    fn person_with_no_names_falls_back_to_id_as_name() {
        let mut p = person("person-001");
        p.given_names = vec![];
        p.family_names = vec![];
        let lookup = InMemoryContributorLookup::empty().with_person(p);
        let agent = resolve_agent("person-001", &lookup);
        assert_eq!(agent.name, "person-001");
        assert_eq!(agent.given_name, None);
        assert_eq!(agent.family_name, None);
    }
}
