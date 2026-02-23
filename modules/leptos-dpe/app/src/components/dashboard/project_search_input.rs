use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query};
use leptos_use::use_debounce_fn;
use mosaic_tiles::icon::{AppStore, Icon, List, Search};

use crate::domain::ProjectQuery;

#[component]
pub fn ProjectSearchInput() -> impl IntoView {
    let query = use_query::<ProjectQuery>();
    let navigate = use_navigate();

    // Get initial search value from URL
    let current_query = query.get().unwrap_or_default();
    let initial_search = current_query.search();

    // Create local signal for input value
    let (search_input, set_search_input) = signal(initial_search.clone());

    // Create debounced function with 300ms delay
    let debounced_navigate = use_debounce_fn(
        move || {
            let search_value = search_input.get();
            let current = query.get().unwrap_or_default();

            // Build new query string with updated search
            let new_query = ProjectQuery {
                search: if search_value.is_empty() {
                    None
                } else {
                    Some(search_value)
                },
                ongoing: current.ongoing,
                finished: current.finished,
                page: Some(1), // Reset to page 1 when search changes
            };

            // Navigate with new query string
            navigate(&new_query.to_query_string(), Default::default());
        },
        300.0,
    );

    view! {
        <div class="w-full p-4 border border-gray-200 rounded-xl overflow-hidden">
        <form
                method="get"
                action="/projects"
                class="flex items-center"
            >

                <label class="input flex-1 mr-2">
                    <Icon icon=Search class="h-6 text-neutral-400" />
                    <input
                        type="search"
                        name="search"
                        placeholder="Search projects..."
                        class="grow"
                        prop:value=search_input
                        on:input=move |ev| {
                            set_search_input.set(event_target_value(&ev));
                            debounced_navigate();
                        }
                    />
                </label>



            <a class="btn btn-ghost" href="/to-do">
                    <Icon icon=AppStore class="w-5 h-5" />
                </a>

            <a class="btn btn-ghost" href="/to-do">
                    <Icon icon=List class="w-5 h-5" />
                </a>

        <button
                    type="submit"
                    class="btn btn-primary btn-sm"
                >
                    "Search"
                </button>
            </form>
        </div>
    }
}
