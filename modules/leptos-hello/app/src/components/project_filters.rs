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
    let other = current_query.other();
    let search = current_query.search();

    // Helper function to build URL with one parameter toggled
    let build_url = |toggle_param: &str| {
        let new_query = ProjectQuery {
            ongoing: Some(if toggle_param == "ongoing" { !ongoing } else { ongoing }),
            finished: Some(if toggle_param == "finished" { !finished } else { finished }),
            other: Some(if toggle_param == "other" { !other } else { other }),
            search: if search.is_empty() { None } else { Some(search.clone()) },
            page: Some(1),
        };
        format!("/projects{}", new_query.to_query_string())
    };

    // Filter checkbox data
    let filters = [
        ("ongoing", "Ongoing", ongoing),
        ("finished", "Finished", finished),
        ("other", "other", other),
    ];

    view! {
        <div class="flex flex-col gap-4">
            // Status filter checkboxes
            <div class="flex gap-4 items-center">
                <span class="font-semibold">"Filter by Status:"</span>
                {filters.iter().map(|(param, label, checked)| {
                    view! {
                        <a href=build_url(param) class="flex items-center gap-2 cursor-pointer hover:opacity-80">
                            <input
                                type="checkbox"
                                class="checkbox checkbox-primary pointer-events-none"
                                checked=*checked
                            />
                            <span>{*label}</span>
                        </a>
                    }
                }).collect_view()}
            </div>

            // Search form
            <form
                method="get"
                action="/projects"
                class="flex gap-4 items-center"
            >
                {filters.iter().map(|(param, _, checked)| {
                    view! {
                        <input type="hidden" name=*param value=checked.to_string() />
                    }
                }).collect_view()}
                <span class="font-semibold">"Search:"</span>
                <input
                    type="text"
                    name="search"
                    class="input input-bordered input-primary w-full max-w-xs"
                    placeholder="Search projects..."
                    value=search
                />
                <button
                    type="submit"
                    class="btn btn-primary btn-sm"
                >
                    "Search"
                </button>
            </form>

            // Reset link
            <div>
                <a href="/projects" class="btn btn-primary btn-sm">
                    "Reset Search and Filter"
                </a>
            </div>
        </div>
    }
}
