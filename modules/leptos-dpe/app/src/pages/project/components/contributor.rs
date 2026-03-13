use leptos::prelude::*;

use super::info_card::InfoCard;
use super::person::PersonView;
use crate::domain::ResolvedContributor;

#[component]
pub fn Contributor(contributor: ResolvedContributor) -> impl IntoView {
    view! {
        <InfoCard>
            {match contributor {
                ResolvedContributor::Person { person, affiliations, roles } => {
                    view! { <PersonView person=person affiliations=affiliations roles=roles /> }
                        .into_any()
                }
                ResolvedContributor::Organization { org, roles } => {
                    view! {
                        <div class="font-medium">
                            <a
                                href=org.url
                                target="_blank"
                                rel="noopener noreferrer"
                                class="text-primary hover:underline"
                            >
                                {org.name}
                            </a>
                        </div>
                        {roles.map(|r| view! { <div class="text-gray-600">{r}</div> })}
                    }
                        .into_any()
                }
                ResolvedContributor::Unknown { id, roles } => {
                    view! {
                        <div class="italic text-base-content/70">
                            {format!("Contributor not found: {id}")}
                        </div>
                        {roles.map(|r| view! { <div class="text-gray-600">{r}</div> })}
                    }
                        .into_any()
                }
            }}
        </InfoCard>
    }
}
