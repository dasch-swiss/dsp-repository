use leptos::prelude::*;
use mosaic_tiles::button::*;
use mosaic_tiles::popover::*;

#[component]
pub fn PopoverAnatomy() -> impl IntoView {
    view! {
        <Popover id="anatomy-popover">
            <PopoverTrigger>
                <Button />
            </PopoverTrigger>
            <PopoverContent />
        </Popover>
    }
}
