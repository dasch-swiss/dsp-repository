use leptos::prelude::*;
use mosaic_tiles::link::Link;

#[component]
pub fn BasicExample() -> impl IntoView {
    view! {
        <div class="flex gap-4 items-center">
            <Link href="/about">"About Us"</Link>
            <Link href="/contact">"Contact"</Link>
            <Link href="/blog">"Blog"</Link>
        </div>
    }
}
