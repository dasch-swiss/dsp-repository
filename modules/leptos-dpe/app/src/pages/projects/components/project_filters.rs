use leptos::prelude::*;
use leptos_router::hooks::use_query;

use crate::domain::ProjectQuery;

use super::filter_checkbox_group::FilterCheckboxGroup;

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
            view: current_query.view,
        };
        format!("/projects{}", new_query.to_query_string())
    };

    // Filter checkbox data
    let filters = [("ongoing", "Ongoing", ongoing), ("finished", "Finished", finished)];

    view! {
        <div class="p-4 border border-gray-200 rounded-lg bg-base-100 w-72">
                <h4 class="dpe-title mb-4">Filters</h4>
        <div class="space-y-4">
            <FilterCheckboxGroup
                title="Status"
                items=filters.iter().map(|(param, label, checked)| {
                    (label.to_string(), *checked, build_url(param))
                }).collect()
            />
        <div class="border-t border-gray-200"></div>

        <div class="dpe-subtitle">Other filters TODO</div>
        </div>
        </div>
    }
}
