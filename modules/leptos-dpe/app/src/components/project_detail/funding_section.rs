use leptos::prelude::*;

use crate::components::{InfoCard, OrganizationName};
use crate::domain::Funding;

#[component]
pub fn FundingSection(funding: Funding) -> impl IntoView {
    view! {
        <div id="funding" class="space-y-2">
            <h3 class="text-base font-semibold">"Funding"</h3>
            {match funding {
                Funding::Grants(grants) => {
                    let grants_clone = grants.clone();
                    view! {
                        <div class="space-y-2">
                            <div class="text-sm font-semibold">"Grants"</div>
                            {grants_clone
                                .into_iter()
                                .map(|grant| {
                                    view! {
                                        <InfoCard>
                                            <div class="text-sm">
                                                {grant
                                                    .funders
                                                    .into_iter()
                                                    .enumerate()
                                                    .map(|(i, funder_id)| {
                                                        view! {
                                                            <span>
                                                                {if i > 0 { ", " } else { "" }}
                                                                <OrganizationName organization_id=funder_id />
                                                            </span>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </div>
                                            {grant
                                                .number
                                                .map(|number| {
                                                    view! {
                                                        <div class="text-sm">
                                                            "Grant: " {number}
                                                        </div>
                                                    }
                                                })}
                                            {grant
                                                .name
                                                .map(|name| {
                                                    view! {
                                                        <div class="text-sm">
                                                            {name}
                                                        </div>
                                                    }
                                                })}
                                            {grant
                                                .url
                                                .map(|url| {
                                                    view! {
                                                        <a href=url class="text-sm text-blue-600 hover:underline flex items-center gap-1" target="_blank">
                                                            "More info"
                                                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                                                            </svg>
                                                        </a>
                                                    }
                                                })}
                                        </InfoCard>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                        .into_any()
                }
                Funding::Text(text) => {
                    view! {
                        <div class="text-sm text-base-content/70">{text}</div>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}
