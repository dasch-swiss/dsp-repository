use leptos::prelude::*;
use mosaic_tiles::button::*;
use mosaic_tiles::popover::*;

#[component]
pub fn MultipleExample() -> impl IntoView {
    view! {
        <div class="flex gap-4">
            <Popover id="popover-1">
                <PopoverTrigger>
                    <Button>"Popover 1"</Button>
                </PopoverTrigger>
                <PopoverContent>
                    <div class="p-4">
                        <p>"Content for first popover."</p>
                    </div>
                </PopoverContent>
            </Popover>
            <Popover id="popover-2">
                <PopoverTrigger>
                    <Button variant=ButtonVariant::Secondary>"Popover 2"</Button>
                </PopoverTrigger>
                <PopoverContent>
                    <div class="p-4">
                        <p>"Content for second popover."</p>
                    </div>
                </PopoverContent>
            </Popover>
            <Popover id="popover-3">
                <PopoverTrigger>
                    <Button variant=ButtonVariant::Outline>"Popover 3"</Button>
                </PopoverTrigger>
                <PopoverContent>
                    <div class="p-4">
                        <p>"Content for third popover."</p>
                    </div>
                </PopoverContent>
            </Popover>
        </div>
    }
}
