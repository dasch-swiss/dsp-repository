use leptos::prelude::*;
use leptos_router::hooks::use_query;
use mosaic_tiles::icon::{Icon, Search};

use crate::domain::ProjectQuery;

#[component]
pub fn ProjectSearchInput() -> impl IntoView {
    let query = use_query::<ProjectQuery>();
    let current_query = query.get().unwrap_or_default();

    view! {
        <form method="get" action="/projects" class="flex items-center flex-1">
            // Preserve existing filter and view state across search submissions
            {current_query
                .ongoing
                .map(|v| view! { <input type="hidden" name="ongoing" value=v.to_string() /> })}
            {current_query
                .finished
                .map(|v| view! { <input type="hidden" name="finished" value=v.to_string() /> })}
            {current_query
                .view
                .map(|v| view! { <input type="hidden" name="view" value=v.to_string() /> })}

            <label class="input flex-1 mr-2">
                <Icon icon=Search class="h-6 text-neutral-400" />
                <input
                    type="search"
                    name="search"
                    placeholder="Search projects..."
                    class="grow"
                    value=current_query.search.unwrap_or_default()
                />
            </label>

            <button type="submit" class="btn btn-primary btn-sm">
                "Search"
            </button>
        </form>
    }
}
