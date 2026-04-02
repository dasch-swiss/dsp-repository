use leptos::prelude::*;

#[component]
pub fn DataLanguageSection(data_languages: Vec<String>) -> impl IntoView {
    (!data_languages.is_empty()).then(|| {
        view! {
            <div>
                <h3 class="dpe-subtitle">"Data Languages"</h3>
                <div class="flex flex-wrap gap-1.5">
                    {data_languages
                        .into_iter()
                        .map(|l| {
                            view! {
                                <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-neutral-100 text-neutral-700">
                                    {l}
                                </span>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
        }
    })
}
