use leptos::prelude::*;

use crate::components::project_detail::publication_year::PublicationYear;

#[component]
pub fn ProjectMetadata(data_publication_year: Option<String>) -> impl IntoView {
    view! {
        <div id="project-details">
            <h3 class="text-base font-semibold mb-3">"Project Details"</h3>
            <div class="space-y-2">
                <PublicationYear year=data_publication_year />
            </div>
        </div>
    }
}
