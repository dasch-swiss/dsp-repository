use leptos::prelude::*;

use crate::components::{ProjectPagination, ProjectSearchInput};
use crate::domain::ProjectQuery;

#[component]
pub fn ProjectSearch(query: ProjectQuery) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-4">
            // Filters component - reads URL parameters directly
            <ProjectSearchInput />

            // Pagination - plain HTML links (MPA style)

        </div>
    }
}
