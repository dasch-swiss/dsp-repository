use dpe_core::ResolvedContributor;
use maud::{html, Markup};

use super::info_card::info_card;
use super::person::person_view;

/// Render a single resolved contributor (person, organization, or unknown) in
/// an info card.
pub fn contributor(contributor: &ResolvedContributor) -> Markup {
    info_card(match contributor {
        ResolvedContributor::Person { person, affiliations, roles } => {
            person_view(person, affiliations, roles.as_deref(), true)
        }
        ResolvedContributor::Organization { org, roles } => html! {
            div class="font-medium" {
                a href=(org.url) target="_blank" rel="noopener noreferrer" class="text-primary hover:underline" {
                    (org.name)
                }
            }
            @if let Some(r) = roles {
                div class="text-gray-600" { (r) }
            }
        },
        ResolvedContributor::Unknown { id, roles } => html! {
            div class="italic text-neutral-500" { (format!("Contributor not found: {id}")) }
            @if let Some(r) = roles {
                div class="text-gray-600" { (r) }
            }
        },
    })
}

#[cfg(test)]
mod tests {
    use dpe_core::organization::Organization;

    use super::*;
    use crate::test_support::sample_person_contributor;

    #[test]
    fn person_contributor_renders_name() {
        let out = contributor(&sample_person_contributor()).into_string();
        assert!(out.contains("Ada Lovelace"), "{out}");
        assert!(out.contains("card card-bordered"), "wrapped in info card: {out}");
    }

    #[test]
    fn organization_contributor_links_out() {
        let c = ResolvedContributor::Organization {
            org: Organization {
                id: "organization-001".to_string(),
                name: "ACME".to_string(),
                same_as: vec![],
                url: "https://acme.example".to_string(),
                address: None,
                email: None,
                alternative_name: None,
            },
            roles: Some("Funder".to_string()),
        };
        let out = contributor(&c).into_string();
        assert!(out.contains(r#"href="https://acme.example""#), "{out}");
        assert!(out.contains("ACME"), "{out}");
        assert!(out.contains("Funder"), "{out}");
    }

    #[test]
    fn unknown_contributor_shows_id() {
        let c = ResolvedContributor::Unknown { id: "person-999".to_string(), roles: None };
        let out = contributor(&c).into_string();
        assert!(out.contains("Contributor not found: person-999"), "{out}");
    }
}
