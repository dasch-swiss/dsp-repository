use maud::{html, Markup};
use mosaic_tiles::icon::{icon, Mail};
use shared_metadata::organization::Organization;
use shared_metadata::person::Person;

/// Shared name + roles + job-titles block for a resolved [`Person`].
fn person_name_and_roles(person: &Person, roles: Option<&str>) -> Markup {
    let full_name = format!("{} {}", person.given_names.join(" "), person.family_names.join(" "));
    // Link the name to the person's ORCID when available, otherwise fall back to
    // their first `sameAs` reference (e.g. an institutional profile page).
    let link_url = person
        .same_as
        .iter()
        .find(|r| r.type_ == "ORCID")
        .or_else(|| person.same_as.first())
        .map(|r| r.url.as_str());
    html! {
        div class="font-medium" {
            @match link_url {
                Some(url) => a   href=(url)
                    target="_blank"
                    rel="noopener noreferrer"
                    class="text-primary hover:underline"
                { (full_name) }
                None => span { (full_name) }
            }
        }
        @if let Some(r) = roles {
            div class="text-gray-600" { (r) }
        }
        @if !person.job_titles.is_empty() {
            div class="text-gray-600" { (person.job_titles.join(", ")) }
        }
    }
}

/// A `mailto:` link with an envelope icon.
fn email_link(addr: &str) -> Markup {
    html! {
        a   href=(format!("mailto:{addr}"))
            class="text-primary hover:underline inline-flex items-center gap-1 mt-1"
        { (icon(Mail, "w-4 h-4")) (addr) }
    }
}

/// Render a person looked up by ID. Affiliations are resolved (by ID) to their
/// organization names. Used where the caller only has an ID (e.g. contact).
pub fn person(person_id: &str, roles: Option<&str>, show_email: bool) -> Markup {
    match dpe_core::load_person(person_id) {
        Some(person) => html! {
            (person_name_and_roles(&person, roles))
            @for org_id in &person.affiliations { (affiliation_name(org_id)) }
            @if show_email {
                @if let Some(addr) = &person.email { (email_link(addr)) }
            }
        },
        None => html! {
            div class="italic text-neutral-500" { "Person not found" }
        },
    }
}

/// Render an organization name (by ID) as an affiliation line.
pub fn affiliation_name(org_id: &str) -> Markup {
    match dpe_core::load_organization(org_id) {
        Some(o) => html! {
            div class="text-gray-600" { (o.name) }
        },
        None => html! {},
    }
}

/// Render a person with pre-resolved affiliation organizations. No lookups —
/// all data is supplied by the caller (the contributor resolver).
pub fn person_view(person: &Person, affiliations: &[Organization], roles: Option<&str>, show_email: bool) -> Markup {
    html! {
        (person_name_and_roles(person, roles))
        @for org in affiliations {
            div class="text-gray-600" { (org.name) }
        }
        @if show_email {
            @if let Some(addr) = &person.email { (email_link(addr)) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{sample_organization, sample_person};

    #[test]
    fn person_view_renders_name_roles_and_affiliation() {
        let p = sample_person();
        let orgs = vec![sample_organization()];
        let out = person_view(&p, &orgs, Some("Author"), false).into_string();
        assert!(out.contains("Ada Lovelace"), "{out}");
        assert!(out.contains("Author"), "role: {out}");
        assert!(out.contains("Researcher"), "job title: {out}");
        assert!(out.contains("Sample University"), "affiliation: {out}");
    }

    #[test]
    fn person_view_shows_email_only_when_requested() {
        let p = sample_person();
        let shown = person_view(&p, &[], None, true).into_string();
        assert!(shown.contains("mailto:ada@example.org"), "{shown}");
        assert!(shown.contains(r#"class="icon w-4 h-4""#), "missing mosaic icon: {shown}");
        assert!(
            !shown.contains(r#"stroke="currentColor""#),
            "raw stroke svg, not the mosaic helper: {shown}"
        );
        assert!(!person_view(&p, &[], None, false).into_string().contains("mailto:"));
    }

    #[test]
    fn unknown_person_renders_not_found() {
        let out = person("person-missing", None, false).into_string();
        assert!(out.contains("Person not found"), "{out}");
    }

    #[test]
    fn name_links_to_orcid_when_present() {
        use shared_metadata::models::AuthorityFileReference;
        let mut p = sample_person();
        p.same_as = vec![
            AuthorityFileReference {
                type_: "URL".to_string(),
                url: "https://example.org/profile".to_string(),
                text: None,
            },
            AuthorityFileReference {
                type_: "ORCID".to_string(),
                url: "https://orcid.org/0000-0002-1825-0097".to_string(),
                text: None,
            },
        ];
        let out = person_view(&p, &[], None, false).into_string();
        assert!(out.contains(r#"href="https://orcid.org/0000-0002-1825-0097""#), "{out}");
    }

    #[test]
    fn name_links_to_sameas_url_when_no_orcid() {
        use shared_metadata::models::AuthorityFileReference;
        let mut p = sample_person();
        p.same_as = vec![AuthorityFileReference {
            type_: "URL".to_string(),
            url: "https://kunstgeschichte.philhist.unibas.ch/en/persons/martinez-ruiperez-antonia/".to_string(),
            text: None,
        }];
        let out = person_view(&p, &[], None, false).into_string();
        assert!(
            out.contains(r#"href="https://kunstgeschichte.philhist.unibas.ch/en/persons/martinez-ruiperez-antonia/""#),
            "{out}"
        );
    }
}
