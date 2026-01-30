use leptos::prelude::*;
use mosaic_tiles::link::Link;

#[component]
pub fn TargetAttributeExample() -> impl IntoView {
    view! {
        <div class="flex flex-col gap-4">
            <div class="flex gap-4 items-center">
                <Link href="/link">"Default (same frame)"</Link>
                <span class="text-sm text-gray-500">"No target attribute"</span>
            </div>
            <div class="flex gap-4 items-center">
                <Link href="/link" target="_self">
                    "Target: _self"
                </Link>
                <span class="text-sm text-gray-500">"Opens in same frame"</span>
            </div>
            <div class="flex gap-4 items-center">
                <Link href="/link" target="_blank" rel="noopener noreferrer">
                    "Target: _blank"
                </Link>
                <span class="text-sm text-gray-500">"Opens in new tab/window"</span>
            </div>
            <div class="flex gap-4 items-center">
                <Link href="/link" target="_parent">
                    "Target: _parent"
                </Link>
                <span class="text-sm text-gray-500">"Opens in parent frame"</span>
            </div>
            <div class="flex gap-4 items-center">
                <Link href="/link" target="_top">
                    "Target: _top"
                </Link>
                <span class="text-sm text-gray-500">"Opens in top-level frame"</span>
            </div>
        </div>
    }
}
