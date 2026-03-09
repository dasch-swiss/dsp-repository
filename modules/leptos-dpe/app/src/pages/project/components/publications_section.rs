use leptos::prelude::*;
use mosaic_tiles::icon::{Export, Icon};
use mosaic_tiles::link::Link;

use crate::domain::Publication;
use crate::pages::project::components::info_card::InfoCard;

#[component]
pub fn PublicationsSection(publications: Vec<Publication>) -> impl IntoView {
    view! {
        <div class="scroll-mt-52">
            <h3 class="dpe-subtitle">"Publications"</h3>
            <div class="space-y-2 text-sm">
                {publications
                    .into_iter()
                    .map(|pub_| {
                        view! {
                            <InfoCard>
                                {(!pub_.text.is_empty())
                                    .then(|| {
                                        view! { <span>{pub_.text.clone()} " "</span> }.into_any()
                                    })}
                                {pub_
                                    .pid
                                    .as_ref()
                                    .map(|pid| {
                                        let href = pid.url.clone();
                                        let text = pid
                                            .text
                                            .clone()
                                            .unwrap_or_else(|| pid.url.clone());
                                        view! {
                                            <span class="ml-2">
                                                <Link href=href>
                                                    {text} <Icon icon=Export class="w-3 h-3" />
                                                </Link>
                                            </span>
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
