use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};
use mosaic_tiles::button_group::*;

#[component]
pub fn DisabledExample() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <ButtonGroup>
                <Button variant=ButtonVariant::Outline>"Active"</Button>
                <Button variant=ButtonVariant::Outline disabled=true>
                    "Disabled"
                </Button>
                <Button variant=ButtonVariant::Outline>"Active"</Button>
            </ButtonGroup>
        </div>
    }
}
