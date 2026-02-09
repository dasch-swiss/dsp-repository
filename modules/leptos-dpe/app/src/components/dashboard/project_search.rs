use leptos::prelude::*;

use crate::components::{ProjectFilters, ProjectPagination};
use crate::domain::ProjectQuery;

#[component]
pub fn ProjectSearch(nr_pages: i32, total_items: i32, query: ProjectQuery) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-4">
            // Filters component - reads URL parameters directly
            <ProjectFilters />
            <ProjectPagination nr_pages=nr_pages total_items=total_items query=query />

            // Pagination - plain HTML links (MPA style)

        </div>
    }
}
