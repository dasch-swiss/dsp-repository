use leptos::prelude::*;

use crate::components::{ProjectSearchInput};
use crate::domain::ProjectQuery;

#[component]
pub fn ProjectSearch(query: ProjectQuery) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-4">
            <ProjectSearchInput />
        </div>
    }
}
