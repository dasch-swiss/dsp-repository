use leptos::prelude::*;
use mosaic_tiles::icon::{Icon, IconGitHub, Star};

#[component]
pub fn HeaderLinks() -> impl IntoView {
    view! {
        <ul class="flex items-center gap-4">
            <li>
                <a href="/to-do" class="btn btn-ghost">
                <Icon icon=IconGitHub class="w-5 h-5" />
                    Help
            </a>
            </li>

            <li>
                <a class="btn btn-primary" href="/to-do">
                    Deposit Data at Dasch
                    <Icon icon=IconGitHub class="w-5 h-5" />
                </a>
            </li>
        </ul>
    }
}
