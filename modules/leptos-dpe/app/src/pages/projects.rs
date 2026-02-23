use leptos::prelude::*;
use leptos_router::hooks::use_query;

use crate::components::{ProjectFilters, ProjectList, ProjectSearchInput};
use crate::domain::ProjectQuery;

#[component]
pub fn ProjectsPage() -> impl IntoView {
    // Use Leptos query for reading URL query parameters
    let query = use_query::<ProjectQuery>();

    view! {
        <div class="flex gap-4">
            <ProjectFilters />

            <div class="flex-1 flex flex-col gap-4">
                <ProjectSearchInput />

                <ProjectList query=query />
            </div>
        </div>
    }
}
