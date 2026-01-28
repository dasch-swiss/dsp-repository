use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn DataBrowserPage() -> impl IntoView {
    let params = use_params_map();
    let selected_id = move || params.read().get("id");

    let elements: Vec<String> = (1..=100).map(|i| format!("element {}", i)).collect();

    view! {
        <h1 class="text-2xl mb-4">Data Browser</h1>
        <div class="flex">
            <div style="height: 70vh; overflow: auto">
                {elements.into_iter().enumerate().map(|(i, element)| {
                    let id = i + 1;
                    view!{
                        <a href=format!("/data-browser/element-{}", id)>
                            <div class="cursor-pointer hover:bg-gray-100 p-2">
                                {element}
                            </div>
                        </a>
                    }
                }).collect_view()}
            </div>

            {move || selected_id().map(|id| view! {
                <div class="p-4 flex-1" style="background-color: #aae5ea;">
                    <h2>"Resource view for " {id}</h2>
                </div>
            })}
        </div>
    }
}
