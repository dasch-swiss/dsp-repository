use leptos::prelude::*;
use mosaic_tiles::button::*;
use mosaic_tiles::popover::*;

use crate::components::counter::Counter;

#[component]
pub fn RichContentExample() -> impl IntoView {
    view! {
        <Popover>
            <PopoverTrigger>
                <Button>"Show Info"</Button>
            </PopoverTrigger>
            <PopoverContent>
                <div class="p-4 space-y-3">
                    <h3 class="text-lg font-semibold">"Popover Title"</h3>
                    <p class="text-sm">
                        "This popover contains rich content including a title, description, and actions."
                    </p>

                    <div class="border-t border-neutral-200 pt-3">
                        <p class="text-sm font-medium mb-2">"Interactive Counter Test"</p>
                        <Counter />
                    </div>
                </div>
            </PopoverContent>
        </Popover>
    }
}
