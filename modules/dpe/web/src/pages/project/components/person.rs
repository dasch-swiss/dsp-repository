use dpe_core::organization::Organization;
use dpe_core::person::Person;
use maud::{html, Markup};

/// Shared name + roles + job-titles block for a resolved [`Person`].
fn person_name_and_roles(person: &Person, roles: Option<&str>) -> Markup {
    let full_name = format!("{} {}", person.given_names.join(" "), person.family_names.join(" "));
    let orcid_url = person.same_as.iter().find(|r| r.type_ == "ORCID").map(|r| r.url.as_str());
    html! {
        div class="font-medium" {
            @match orcid_url {
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
        {
            svg xmlns="http://www.w3.org/2000/svg"
                class="w-3 h-3"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
            {
                rect width="20" height="16" x="2" y="4" rx="2";
                path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7";
            }
            (addr)
        }
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
        assert!(person_view(&p, &[], None, true)
            .into_string()
            .contains("mailto:ada@example.org"));
        assert!(!person_view(&p, &[], None, false).into_string().contains("mailto:"));
    }

    #[test]
    fn unknown_person_renders_not_found() {
        let out = person("person-missing", None, false).into_string();
        assert!(out.contains("Person not found"), "{out}");
    }
}
