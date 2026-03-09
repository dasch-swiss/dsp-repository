use leptos::prelude::*;

use super::info_card::InfoCard;
use super::person::Person;
use crate::domain::{get_organization, Attribution};

#[component]
pub fn Contributor(attr: Attribution) -> impl IntoView {
    let id = attr.contributor.clone();
    let roles = (!attr.contributor_type.is_empty()).then(|| attr.contributor_type.join(", "));
    if id.contains("-organization-") {
        view! { <OrgContributor org_id=id roles=roles /> }.into_any()
    } else {
        view! { <PersonContributor person_id=id roles=roles /> }.into_any()
    }
}

#[component]
fn PersonContributor(person_id: String, roles: Option<String>) -> impl IntoView {
    view! {
        <InfoCard>
            <div class="text-sm">
                <Person person_id=person_id roles=roles />
            </div>
        </InfoCard>
    }
}

#[component]
fn OrgContributor(org_id: String, roles: Option<String>) -> impl IntoView {
    let resource = Resource::new(move || org_id.clone(), |id| async move { get_organization(id).await });
    view! {
        <InfoCard>
            <div class="text-sm">
                <Suspense>
                    {move || {
                        let org_opt = resource.get().and_then(|r| r.ok()).flatten();
                        match org_opt {
                            Some(org) => {
                                view! {
                                    <div class="font-medium">
                                        <a
                                            href=org.url.clone()
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            class="text-primary hover:underline"
                                        >
                                            {org.name.clone()}
                                        </a>
                                    </div>
                                    {roles
                                        .clone()
                                        .map(|r| view! { <div class="text-gray-600">{r}</div> })}
                                }
                                    .into_any()
                            }
                            None => {
                                view! {
                                    <div class="italic text-base-content/70">
                                        "Organization not found"
                                    </div>
                                }
                                    .into_any()
                            }
                        }
                    }}
                </Suspense>
            </div>
        </InfoCard>
    }
}
