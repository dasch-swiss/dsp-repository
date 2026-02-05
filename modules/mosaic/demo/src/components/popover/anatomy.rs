use leptos::prelude::*;
use mosaic_tiles::button::*;
use mosaic_tiles::popover::*;

#[component]
pub fn PopoverAnatomy() -> impl IntoView {
    view! {
        <Popover>
            <PopoverTrigger>
                <Button />
            </PopoverTrigger>
            <PopoverContent />
        </Popover>
    }
}
