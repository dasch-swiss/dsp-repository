use leptos::prelude::*;

#[component]
pub fn TypeOfDataSection(type_of_data: Option<Vec<String>>) -> impl IntoView {
    type_of_data.map(|types| {
        view! {
            <div id="type-of-data">
                <h3 class="text-sm font-semibold text-gray-700 mb-2">"Type of Data"</h3>
                <div class="flex flex-wrap gap-1.5">
                    {types
                        .into_iter()
                        .map(|t| {
                            view! {
                                <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-primary-50 text-primary-700">
                                    {t}
                                </span>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
        }
    })
}
