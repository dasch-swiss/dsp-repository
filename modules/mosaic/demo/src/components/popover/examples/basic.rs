use leptos::prelude::*;
use mosaic_tiles::button::*;
use mosaic_tiles::popover::*;

#[component]
pub fn BasicExample() -> impl IntoView {
    view! {
        <Popover id="basic-popover">
            <PopoverTrigger>
                <Button>"Open Popover"</Button>
            </PopoverTrigger>
            <PopoverContent>
                <div class="p-4">
                    <p>"This is a basic popover content."</p>
                </div>
            </PopoverContent>
        </Popover>
    }
}
