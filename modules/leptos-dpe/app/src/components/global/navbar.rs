use leptos::prelude::*;

use crate::components::global::header_links::HeaderLinks;

#[component]
pub fn NavBar() -> impl IntoView {
    view! {
        <div class="navbar bg-base-100 shadow-sm">
            <div class="flex-none">
                <img src="/logo.svg" class="inline h-10 w-10 mr-2" />
            </div>

            <div class="flex-1">
                <a class="btn btn-ghost text-xl" href="/">
                    "DaSCH Metadata Browser"
                </a>
            </div>

            <div class="flex">
                <HeaderLinks />
            </div>
        </div>
    }
}
