use leptos::prelude::*;
use mosaic_tiles::link::Link;

#[component]
pub fn ExternalExample() -> impl IntoView {
    view! {
        <div class="flex gap-4 items-center">
            <Link href="/link" target="_blank" rel="noopener noreferrer">
                "External Link (Demo)"
            </Link>
            <Link href="/link" target="_blank" rel="noopener noreferrer">
                "Opens in New Tab (Demo)"
            </Link>
        </div>
    }
}
