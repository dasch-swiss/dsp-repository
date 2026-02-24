use leptos::prelude::*;

#[component]
pub fn ProjectMetadata(data_publication_year: Option<String>) -> impl IntoView {
    view! {
        <div id="project-details">
            <h3 class="text-base font-semibold mb-3">"Project Details"</h3>
            <div class="space-y-2">
            </div>
        </div>
    }
}
