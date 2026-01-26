use leptos::prelude::*;
use mosaic_tiles::{button::Button, button_group::ButtonGroup};

#[component]
pub fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    view! {
        <ButtonGroup>
            <Button on:click=move |_| set_count.update(|n| *n -= 1)>"-"</Button>
            <Button disabled=true>{count}</Button>
            <Button on:click=move |_| set_count.update(|n| *n += 1)>"+"</Button>
        </ButtonGroup>
    }
}
