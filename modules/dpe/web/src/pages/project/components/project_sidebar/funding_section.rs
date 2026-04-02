use leptos::prelude::*;
use mosaic_tiles::icon::{Icon, LinkExternal};

use super::super::info_card::InfoCard;
use super::super::organization_name::OrganizationName;
use crate::domain::Funding;

#[component]
pub fn FundingSection(funding: Funding) -> impl IntoView {
    view! {
        <div>
            {match funding {
                Funding::Grants(grants) => {
                    let grants_clone = grants.clone();
                    view! {
                        <div class="space-y-2">
                            <div class="dpe-subtitle">"Grants"</div>
                            {grants_clone
                                .into_iter()
                                .map(|grant| {
                                    view! {
                                        <InfoCard>
                                            <div>
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
                                                    view! { <div>"Grant: " {number}</div> }
                                                })}
                                            {grant
                                                .name
                                                .map(|name| {
                                                    view! { <div>{name}</div> }
                                                })}
                                            {grant
                                                .url
                                                .map(|url| {
                                                    view! {
                                                        <a
                                                            href=url
                                                            class="text-primary items-center gap-1"
                                                            target="_blank"
                                                        >
                                                            "More info"
                                                            <Icon icon=LinkExternal class="w-3 h-3" />
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
                    view! { <div class="text-base-content/70">{text}</div> }.into_any()
                }
            }}
        </div>
    }
}
