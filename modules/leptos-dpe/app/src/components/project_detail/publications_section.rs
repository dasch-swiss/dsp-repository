use leptos::prelude::*;

use crate::components::project_detail::info_card::InfoCard;
use crate::domain::Publication;

#[component]
pub fn PublicationsSection(publications: Vec<Publication>) -> impl IntoView {
    view! {
        <div
            id="publications"
            class="bg-base-100 rounded-lg scroll-mt-52"
        >
            <h3 class="font-semibold mb-3">"Publications"</h3>
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
                                                class="link link-primary ml-2"
                                            >
                                                {pid
                                                    .text
                                                    .clone()
                                                    .unwrap_or_else(|| pid.url.clone())}
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
