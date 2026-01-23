use leptos::prelude::*;
use mosaic_tiles::link::*;

#[component]
pub fn LinkAnatomy() -> impl IntoView {
    view! {
        <Link href="/destination">
            "Link Text"
        </Link>
    }
}
