use leptos::prelude::*;
use mosaic_tiles::button::Button;
use mosaic_tiles::button_group::*;

#[component]
pub fn ButtonGroupAnatomy() -> impl IntoView {
    view! {
        <ButtonGroup>
            <Button>"Option 1"</Button>
            <Button>"Option 2"</Button>
            <Button>"Option 3"</Button>
        </ButtonGroup>
    }
}
