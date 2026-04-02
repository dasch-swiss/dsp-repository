use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};
use mosaic_tiles::button_group::ButtonGroup;

#[island]
pub fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    view! {
        <ButtonGroup>
            <Button variant=ButtonVariant::Outline on:click=move |_| set_count.update(|n| *n -= 1)>
                "-"
            </Button>
            <Button variant=ButtonVariant::Outline>{count}</Button>
            <Button variant=ButtonVariant::Outline on:click=move |_| set_count.update(|n| *n += 1)>
                "+"
            </Button>
        </ButtonGroup>
    }
}
