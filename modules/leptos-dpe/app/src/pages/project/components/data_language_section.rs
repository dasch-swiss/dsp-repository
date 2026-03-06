use leptos::prelude::*;
use mosaic_tiles::badge::{Badge, BadgeSize};

#[component]
pub fn DataLanguageSection(data_languages: Vec<String>) -> impl IntoView {
    (!data_languages.is_empty()).then(|| {
        view! {
            <div id="data-languages" class="scroll-mt-52">
                <h3 class="dpe-subtitle">"Data Languages"</h3>
                <div class="flex flex-wrap gap-2">
                    {data_languages
                        .into_iter()
                        .map(|l| {
                            view! { <Badge size=BadgeSize::Small>{l}</Badge> }
                        })
                        .collect_view()}
                </div>
            </div>
        }
    })
}
