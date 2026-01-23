use leptos::prelude::*;
use mosaic_tiles::link::Link;

#[component]
pub fn BasicExample() -> impl IntoView {
    view! {
        <div class="flex gap-4 items-center">
            <Link href="/link">"About Us"</Link>
            <Link href="/link">"Contact"</Link>
            <Link href="/link">"Blog"</Link>
        </div>
    }
}
