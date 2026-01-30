use leptos::prelude::*;
use mosaic_tiles::link::Link;

#[component]
pub fn DisabledExample() -> impl IntoView {
    view! {
        <div class="flex gap-4 items-center">
            <Link href="/link">"Available Link"</Link>
            <Link href="/link" disabled=true>
                "Disabled Link"
            </Link>
            <Link href="/link" as_button=true disabled=true>
                "Disabled Button Link"
            </Link>
        </div>
    }
}
