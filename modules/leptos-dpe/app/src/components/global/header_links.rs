use leptos::prelude::*;
use mosaic_tiles::icon::{Export, Help, Icon};

#[component]
pub fn HeaderLinks() -> impl IntoView {
    view! {
        <ul class="flex items-center gap-4">
            <li>
                <a href="/about" class="btn btn-ghost">
                <Icon icon=Help class="w-5 h-5" />
                    Help
            </a>
            </li>

            <li>
                <a class="btn btn-primary" href="/to-do">
                    Deposit Data at DaSCH
                    <Icon icon=Export class="w-5 h-5" />
                </a>
            </li>
        </ul>
    }
}
