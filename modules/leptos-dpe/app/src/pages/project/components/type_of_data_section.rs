use leptos::prelude::*;
use mosaic_tiles::badge::{Badge, BadgeSize};

#[component]
pub fn TypeOfDataSection(type_of_data: Option<Vec<String>>) -> impl IntoView {
    type_of_data.map(|types| {
        view! {
            <div id="type-of-data">
                <h3 class="dpe-subtitle">"Type of Data"</h3>
                <div class="flex flex-wrap gap-2">
                    {types
                        .into_iter()
                        .map(|t| {
                            view! { <Badge size=BadgeSize::Small>{t}</Badge> }
                        })
                        .collect_view()}
                </div>
            </div>
        }
    })
}
