use leptos::prelude::*;
use leptos_router::hooks::use_query;
use mosaic_tiles::icon::{AppStore, Icon, List, Search};

use crate::domain::ProjectQuery;

#[component]
pub fn ProjectSearchInput() -> impl IntoView {
    let query = use_query::<ProjectQuery>();
    let current_query = query.get().unwrap_or_default();
    let search = current_query.search();

    view! {
        <div class="w-full p-4 border border-gray-200 rounded-xl overflow-hidden">
        <form
                method="get"
                action="/projects"
                class="flex items-center"
            >

                <label class="input flex-1 mr-2">
                    <Icon icon=Search class="h-6 text-neutral-400" />
                    <input type="search" placeholder="Search projects..." class="grow" value=search />
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
