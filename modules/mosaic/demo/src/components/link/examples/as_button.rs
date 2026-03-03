use leptos::prelude::*;
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::link::Link;

#[component]
pub fn AsButtonExample() -> impl IntoView {
    view! {
        <div class="flex gap-4 items-center">
            <Link href="/link" as_button=ButtonVariant::Primary>
                "Primary"
            </Link>
            <Link href="/link" as_button=ButtonVariant::Secondary>
                "Secondary"
            </Link>
            <Link href="/link" as_button=ButtonVariant::Outline>
                "Outline"
            </Link>
            <Link href="/link" as_button=ButtonVariant::Soft>
                "Soft"
            </Link>
            <Link href="/link" as_button=ButtonVariant::Ghost>
                "Ghost"
            </Link>
        </div>
    }
}
