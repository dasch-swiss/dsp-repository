use leptos::prelude::*;

use crate::domain::organization::Organization;
use crate::domain::person::Person as PersonData;

// See `super` module docs for why this is a sync lookup with a wasm32 stub.

/// Renders a person by ID. Used where the caller only has an ID (e.g. legal info
/// sidebar). For bulk contributor rendering, prefer `PersonView` with pre-resolved data.
#[cfg(not(target_arch = "wasm32"))]
#[component]
pub fn Person(person_id: String, roles: Option<String>, #[prop(default = false)] show_email: bool) -> impl IntoView {
    match dpe_core::load_person(&person_id) {
        Some(person) => {
            let affiliation_ids = person.affiliations.clone();
            view! {
                <PersonViewWithAffiliationIds
                    person=person
                    affiliation_ids=affiliation_ids
                    roles=roles
                    show_email=show_email
                />
            }
            .into_any()
        }
        None => view! { <div class="italic text-base-content/70">"Person not found"</div> }.into_any(),
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn Person(person_id: String, roles: Option<String>, #[prop(default = false)] show_email: bool) -> impl IntoView {
    let _ = (person_id, roles, show_email);
}

// Only instantiated by the non-wasm `Person` above. Gated to silence
// dead-code warnings in the wasm32 build.
#[cfg(not(target_arch = "wasm32"))]
#[component]
fn PersonViewWithAffiliationIds(
    person: PersonData,
    affiliation_ids: Vec<String>,
    roles: Option<String>,
    #[prop(default = false)] show_email: bool,
) -> impl IntoView {
    let full_name = format!("{} {}", person.given_names.join(" "), person.family_names.join(" "),);
    let orcid_url = person.same_as.iter().find(|r| r.type_ == "ORCID").map(|r| r.url.clone());
    let job_titles = person.job_titles;
    let email = if show_email { person.email } else { None };

    view! {
        <div class="font-medium">
            {match orcid_url {
                Some(url) => {
                    view! {
                        <a
                            href=url
                            target="_blank"
                            rel="noopener noreferrer"
                            class="text-primary hover:underline"
                        >
                            {full_name}
                        </a>
                    }
                        .into_any()
                }
                None => view! { <span>{full_name}</span> }.into_any(),
            }}
        </div>
        {roles.map(|r| view! { <div class="text-gray-600">{r}</div> })}
        {(!job_titles.is_empty())
            .then(|| view! { <div class="text-gray-600">{job_titles.join(", ")}</div> })}
        {affiliation_ids
            .into_iter()
            .map(|org_id| view! { <AffiliationName org_id=org_id /> })
            .collect_view()}
        {(show_email)
            .then(|| {
                email
                    .map(|addr| {
                        let href = format!("mailto:{}", addr);
                        view! {
                            <a
                                href=href
                                class="text-primary hover:underline inline-flex items-center gap-1 mt-1"
                            >
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    class="w-3 h-3"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <rect width="20" height="16" x="2" y="4" rx="2" />
                                    <path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7" />
                                </svg>
                                {addr}
                            </a>
                        }
                    })
            })}
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[component]
pub fn AffiliationName(org_id: String) -> impl IntoView {
    dpe_core::load_organization(&org_id).map(|o| view! { <div class="text-gray-600">{o.name}</div> })
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn AffiliationName(org_id: String) -> impl IntoView {
    let _ = org_id;
}

/// Renders a person's name (with ORCID link if available), job titles, and affiliations.
/// All data is pre-resolved — no server calls are made from this component.
#[component]
pub fn PersonView(
    person: PersonData,
    affiliations: Vec<Organization>,
    roles: Option<String>,
    #[prop(default = false)] show_email: bool,
) -> impl IntoView {
    let full_name = format!("{} {}", person.given_names.join(" "), person.family_names.join(" "),);
    let orcid_url = person.same_as.iter().find(|r| r.type_ == "ORCID").map(|r| r.url.clone());
    let job_titles = person.job_titles;
    let email = if show_email { person.email } else { None };

    view! {
        <div class="font-medium">
            {match orcid_url {
                Some(url) => {
                    view! {
                        <a
                            href=url
                            target="_blank"
                            rel="noopener noreferrer"
                            class="text-primary hover:underline"
                        >
                            {full_name}
                        </a>
                    }
                        .into_any()
                }
                None => view! { <span>{full_name}</span> }.into_any(),
            }}
        </div>
        {roles.map(|r| view! { <div class="text-gray-600">{r}</div> })}
        {(!job_titles.is_empty())
            .then(|| view! { <div class="text-gray-600">{job_titles.join(", ")}</div> })}
        {affiliations
            .into_iter()
            .map(|org| view! { <div class="text-gray-600">{org.name}</div> })
            .collect_view()}
        {(show_email)
            .then(|| {
                email
                    .map(|addr| {
                        let href = format!("mailto:{}", addr);
                        view! {
                            <a
                                href=href
                                class="text-primary hover:underline inline-flex items-center gap-1 mt-1"
                            >
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    class="w-3 h-3"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <rect width="20" height="16" x="2" y="4" rx="2" />
                                    <path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7" />
                                </svg>
                                {addr}
                            </a>
                        }
                    })
            })}
    }
}
