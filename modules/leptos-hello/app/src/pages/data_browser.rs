use leptos::prelude::*;

#[component]
pub fn DataBrowserPage() -> impl IntoView {
    let elements: Vec<String> = (1..=100).map(|i| format!("element {}", i)).collect();
    view! {
            <h1 class="text-2xl mb-4">Data Browser</h1>
        <div class="flex">
            <div style="height: 70vh; overflow: auto">
                {elements.into_iter().map(|element| view!{ <div>{element}</div>}).collect_view()}
            </div>

            <div class="p-4 flex-1" style="background-color: #aae5ea;">
                <h2>Resource view</h2>
            </div>
        </div>
    }
}
