use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};

#[component]
pub fn VariantsExample() -> impl IntoView {
    view! {
        <div class="flex gap-4 items-center">
            <Button variant=ButtonVariant::Primary>"Primary Button"</Button>
            <Button variant=ButtonVariant::Secondary>"Secondary Button"</Button>
            <Button variant=ButtonVariant::Outline>"Outline Button"</Button>
        </div>
    }
}
