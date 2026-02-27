use leptos::prelude::*;

use crate::components::global::header_links::HeaderLinks;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <div class="bg-base-100 shadow-xs">
            <div class="navbar max-w-7xl mx-auto px-4">
                <a href="/">
                    <img src="/logo.svg" class="inline h-10 w-10 mr-2" />
                </a>

                <div class="flex-1">
                    <a class="btn btn-ghost text-xl px-1" href="/">
                        "DaSCH Metadata Browser"
                    </a>
                </div>

                <div class="flex">
                    <HeaderLinks />
                </div>
            </div>
        </div>
    }
}
