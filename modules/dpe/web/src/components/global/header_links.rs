use leptos::prelude::*;
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::icon::{Export, Help, Icon};
use mosaic_tiles::link::Link;

#[component]
pub fn HeaderLinks() -> impl IntoView {
    view! {
        <ul class="flex items-center gap-4">
            <li>
                <Link href="/dpe/about" as_button=ButtonVariant::Ghost>
                    <Icon icon=Help class="w-5 h-5" />
                    Help
                </Link>
            </li>

            <li>
                <Link href="https://dasch.swiss" as_button=ButtonVariant::Primary target="_blank">
                    Deposit Data at DaSCH
                    <Icon icon=Export class="w-5 h-5" />
                </Link>
            </li>
        </ul>
    }
}
