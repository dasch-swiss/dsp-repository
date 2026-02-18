use leptos::prelude::*;
use leptos_router::hooks::use_query;

use crate::domain::ProjectQuery;

// Regular component for filters and search - uses simple links that reload the page
#[component]
pub fn ProjectFilters() -> impl IntoView {
    // Use Leptos query for reading URL query parameters
    let query = use_query::<ProjectQuery>();
    let current_query = query.get().unwrap_or_default();

    let ongoing = current_query.ongoing();
    let finished = current_query.finished();
    let search = current_query.search();

    // Helper function to build URL with one parameter toggled
    let build_url = |toggle_param: &str| {
        let new_query = ProjectQuery {
            ongoing: Some(if toggle_param == "ongoing" { !ongoing } else { ongoing }),
            finished: Some(if toggle_param == "finished" {
                !finished
            } else {
                finished
            }),
            search: if search.is_empty() { None } else { Some(search.clone()) },
            page: Some(1),
        };
        format!("/projects{}", new_query.to_query_string())
    };

    // Filter checkbox data
    let filters = [("ongoing", "Ongoing", ongoing), ("finished", "Finished", finished)];

    view! {
        <div class="p-4 border border-gray-200 rounded-xl overflow-hidden" style="min-width: 250px">
                <div class="font-bold">Filters</div>
                {filters.iter().map(|(param, label, checked)| {
                    view! {
                        <a href=build_url(param) class="flex items-center gap-2 cursor-pointer hover:opacity-80">
                            <span class="flex-1">{*label}</span>
                            <input
                                type="checkbox"
                                class="checkbox checkbox-sm pointer-events-none"
                                checked=*checked
                            />
                        </a>
                    }
                }).collect_view()}
        </div>
    }
}
