use leptos::prelude::*;

use crate::domain::organization::Organization;
use crate::domain::person::Person as PersonData;
use crate::domain::{get_organization, get_person};

/// Fetches and renders a person by ID. Used where the caller only has an ID (e.g. legal info sidebar).
/// For bulk contributor rendering, prefer `PersonView` with pre-resolved data.
#[component]
pub fn Person(
    person_id: String,
    roles: Option<String>,
    #[prop(default = false)] show_email: bool,
) -> impl IntoView {
    let person_resource =
        Resource::new(move || person_id.clone(), |id| async move { get_person(id).await });

    view! {
        <Suspense>
            {move || {
                let person_opt = person_resource.get().and_then(|result| result.ok()).flatten();
                match person_opt {
                    Some(person) => {
                        let affiliation_ids = person.affiliations.clone();
                        view! {
                            <PersonViewWithAffiliationIds
                                person=person
                                affiliation_ids=affiliation_ids
                                roles=roles.clone()
                                show_email=show_email
                            />
                        }
                            .into_any()
                    }
                    None => {
                        view! { <div class="italic text-base-content/70">"Person not found"</div> }
                            .into_any()
                    }
                }
            }}
        </Suspense>
    }
}

#[component]
fn PersonViewWithAffiliationIds(
    person: PersonData,
    affiliation_ids: Vec<String>,
    roles: Option<String>,
    #[prop(default = false)] show_email: bool,
) -> impl IntoView {
    let full_name =
        format!("{} {}", person.given_names.join(" "), person.family_names.join(" "),);
    let orcid_url =
        person.same_as.iter().find(|r| r.type_ == "ORCID").map(|r| r.url.clone());
    let job_titles = person.job_titles.clone();
    let email = person.email.clone();

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

#[component]
pub fn AffiliationName(org_id: String) -> impl IntoView {
    let org_resource =
        Resource::new(move || org_id.clone(), |id| async move { get_organization(id).await });

    view! {
        <Suspense>
            {move || {
                let name = org_resource.get().and_then(|r| r.ok()).flatten().map(|o| o.name);
                name.map(|n| view! { <div class="text-gray-600">{n}</div> })
            }}
        </Suspense>
    }
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
    let full_name =
        format!("{} {}", person.given_names.join(" "), person.family_names.join(" "),);
    let orcid_url =
        person.same_as.iter().find(|r| r.type_ == "ORCID").map(|r| r.url.clone());
    let job_titles = person.job_titles.clone();
    let email = person.email.clone();

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
