use leptos::prelude::*;

use crate::components::{OrganizationName, UrlBadge};
use crate::domain::Funding;

#[component]
pub fn FundingSection(funding: Funding) -> impl IntoView {
    view! {
        <div
            id="funding"
            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
        >
            <h3 class="text-xl font-bold mb-3">"Funding"</h3>
            {match &funding {
                Funding::Grants(grants) => {
                    view! {
                        <div class="grid md:grid-cols-2 lg:grid-cols-3 gap-4">
                            {grants
                                .iter()
                                .map(|grant| {
                                    view! {
                                        <div class="border-l-4 border-primary pl-4 space-y-2">
                                            {grant
                                                .name
                                                .as_ref()
                                                .map(|name| {
                                                    view! {
                                                        <div class="font-semibold">
                                                            {name.clone()}
                                                        </div>
                                                    }
                                                })}
                                            {grant
                                                .number
                                                .as_ref()
                                                .map(|number| {
                                                    view! {
                                                        <div class="text-sm">
                                                            "Grant Number: " {number.clone()}
                                                        </div>
                                                    }
                                                })}
                                            <div class="text-sm">
                                                "Funders: "
                                                {grant
                                                    .funders
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(i, funder_id)| {
                                                        view! {
                                                            <span>
                                                                {if i > 0 { ", " } else { "" }}
                                                                <OrganizationName organization_id=funder_id
                                                                    .clone() />
                                                            </span>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </div>
                                            {grant
                                                .url
                                                .as_ref()
                                                .map(|url| {
                                                    view! { <UrlBadge url=url.clone() /> }
                                                })}
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                        .into_any()
                }
                Funding::Text(text) => {
                    view! {
                        <div class="text-base-content/70">{text.clone()}</div>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}
