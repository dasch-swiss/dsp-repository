use leptos::prelude::*;

#[component]
pub fn TypeOfDataSection(type_of_data: Option<Vec<String>>) -> impl IntoView {
    type_of_data.map(|types| {
        view! {
            <div
                id="type-of-data"
            >
                <h3 class="font-semibold mb-2">"Type of Data"</h3>
                <div class="flex flex-wrap gap-2">
                    {types
                        .iter()
                        .map(|t| {
                            view! {
                                <span class="badge badge-primary text-xs">
                                    {t.clone()}
                                </span>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
        }
    })
}
