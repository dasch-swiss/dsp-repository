use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};

#[component]
pub fn DisabledExample() -> impl IntoView {
    view! {
        <div class="flex gap-4 items-center">
            <Button disabled=true variant=ButtonVariant::Primary>
                "Disabled Primary"
            </Button>
            <Button disabled=true variant=ButtonVariant::Secondary>
                "Disabled Secondary"
            </Button>
        </div>
    }
}
