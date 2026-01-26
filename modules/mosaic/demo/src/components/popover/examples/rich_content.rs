use leptos::prelude::*;
use mosaic_tiles::button::*;
use mosaic_tiles::popover::*;

#[component]
pub fn RichContentExample() -> impl IntoView {
    let count = RwSignal::new(0);

    let increment = move |_| {
        count.update(|c| *c += 1);
    };

    let decrement = move |_| {
        count.update(|c| *c -= 1);
    };

    view! {
        <Popover>
            <PopoverTrigger>
                <Button>"Show Info"</Button>
            </PopoverTrigger>
            <PopoverContent>
                <div class="p-4 space-y-3">
                    <h3 class="text-lg font-semibold">"Popover Title"</h3>
                    <p class="text-sm">"This popover contains rich content including a title, description, and actions."</p>

                    <div class="border-t border-gray-200 pt-3">
                        <p class="text-sm font-medium mb-2">"Interactive Counter Test"</p>
                        <div class="flex items-center gap-3">
                            <Button variant=ButtonVariant::Outline on:click=decrement>"-"</Button>
                            <span class="text-lg font-semibold min-w-[3ch] text-center">{count}</span>
                            <Button variant=ButtonVariant::Outline on:click=increment>"+"</Button>
                        </div>
                    </div>

                    <div class="flex gap-2 pt-2">
                        <Button variant=ButtonVariant::Primary>"Action"</Button>
                        <Button variant=ButtonVariant::Secondary>"Cancel"</Button>
                    </div>
                </div>
            </PopoverContent>
        </Popover>
    }
}
