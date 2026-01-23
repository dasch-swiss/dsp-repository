use leptos::prelude::*;
use mosaic_tiles::link::Link;

#[component]
pub fn ExternalExample() -> impl IntoView {
    view! {
        <div class="flex gap-4 items-center">
            <Link href="https://example.com" target="_blank" rel="noopener noreferrer">
                "External Link"
            </Link>
            <Link href="https://github.com" target="_blank" rel="noopener noreferrer">
                "GitHub"
            </Link>
        </div>
    }
}
