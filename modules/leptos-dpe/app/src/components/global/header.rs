use leptos::prelude::*;

use crate::components::global::header_links::HeaderLinks;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <div class="bg-white shadow-xs">
            <div class="flex items-center py-2 max-w-7xl mx-auto px-4">
                <a href="/">
                    <img src="/logo.svg" class="inline h-10 w-10 mr-2" />
                </a>

                <div class="flex-1">
                    <a
                        class="inline-flex items-center text-xl px-1 rounded-md cursor-pointer hover:bg-neutral-100"
                        href="/"
                    >
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
