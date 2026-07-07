use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};
use mosaic_tiles::button_group::*;

#[component]
pub fn SizesExample() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <div>
                <p class="text-sm text-neutral-600 mb-2">"Small"</p>
                <ButtonGroup size=ButtonGroupSize::Small>
                    <Button variant=ButtonVariant::Outline>"Small"</Button>
                    <Button variant=ButtonVariant::Outline>"Button"</Button>
                    <Button variant=ButtonVariant::Outline>"Group"</Button>
                </ButtonGroup>
            </div>

            <div>
                <p class="text-sm text-neutral-600 mb-2">"Medium (Default)"</p>
                <ButtonGroup size=ButtonGroupSize::Medium>
                    <Button variant=ButtonVariant::Outline>"Medium"</Button>
                    <Button variant=ButtonVariant::Outline>"Button"</Button>
                    <Button variant=ButtonVariant::Outline>"Group"</Button>
                </ButtonGroup>
            </div>

            <div>
                <p class="text-sm text-neutral-600 mb-2">"Large"</p>
                <ButtonGroup size=ButtonGroupSize::Large>
                    <Button variant=ButtonVariant::Outline>"Large"</Button>
                    <Button variant=ButtonVariant::Outline>"Button"</Button>
                    <Button variant=ButtonVariant::Outline>"Group"</Button>
                </ButtonGroup>
            </div>
        </div>
    }
}
