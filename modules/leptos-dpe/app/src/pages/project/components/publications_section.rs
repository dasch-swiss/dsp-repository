use crate::domain::Publication;
use crate::pages::project::components::info_card::InfoCard;
use leptos::prelude::*;
use mosaic_tiles::icon::{Export, Icon};

#[component]
pub fn PublicationsSection(publications: Vec<Publication>) -> impl IntoView {
    view! {
        <div
            id="publications"
            class="bg-base-100 rounded-lg scroll-mt-52"
        >
            <h3 class="dpe-subtitle">"Publications"</h3>
            <div class="space-y-2 text-sm">
                {publications
                    .into_iter()
                    .map(|pub_| {
                        view! {
                            <InfoCard>
                                {(!pub_.text.is_empty())
                                    .then(|| {
                                        view! {
                                            <span>{pub_.text.clone()} " "</span>
                                        }
                                            .into_any()
                                    })}
                                {pub_
                                    .pid
                                    .as_ref()
                                    .map(|pid| {
                                        view! {
                                            <a
                                                href=pid.url.clone()
                                                class="link link-primary ml-2 inline-flex gap-1"
                                            >
                                                {pid
                                                    .text
                                                    .clone()
                                                    .unwrap_or_else(|| pid.url.clone())}
                    <Icon icon=Export class="w-3 h-3" />

                                            </a>
                                        }
                                    })}
                            </InfoCard>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}
