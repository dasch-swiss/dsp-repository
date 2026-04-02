use leptos::prelude::*;

use crate::components::global::header_links::HeaderLinks;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <div class="bg-white shadow-xs">
            <div class="flex items-center py-2 dpe-max-layout-width mx-auto px-4">
                <a href="/" aria-label="DaSCH Metadata Browser home">
                    <img src="/logo.svg" class="inline h-10 w-10 mr-2" alt="DaSCH logo" />
                </a>

                <div class="flex-1">
                    <a class="inline-flex items-center font-bold font-display text-xl" href="/">
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
