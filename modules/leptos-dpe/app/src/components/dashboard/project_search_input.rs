use leptos::prelude::*;
use leptos_router::hooks::use_query;
use mosaic_tiles::icon::{Icon, IconGitHub};

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
                class="flex gap-4 items-center"
            >
                <input
                    type="text"
                    name="search"
                    class="input input-bordered input-primary flex-1"
                    placeholder="Search projects..."
                    value=search
                />
                <button
                    type="submit"
                    class="btn btn-primary btn-sm"
                >
                    "Search"
                </button>

            <a class="btn" href="/to-do">
                    <Icon icon=IconGitHub class="w-5 h-5" />
                </a>

            <a class="btn" href="/to-do">
                    <Icon icon=IconGitHub class="w-5 h-5" />
                </a>
            </form>
        </div>
    }
}
