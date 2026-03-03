use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};
use mosaic_tiles::button_group::*;

#[component]
pub fn BasicExample() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <div class="flex gap-8">
                <div>
                    <p class="text-sm text-neutral-600 mb-2">"Horizontal (Default)"</p>
                    <ButtonGroup>
                        <Button variant=ButtonVariant::Outline>"Left"</Button>
                        <Button variant=ButtonVariant::Outline>"Center"</Button>
                        <Button variant=ButtonVariant::Outline>"Right"</Button>
                    </ButtonGroup>
                </div>

                <div>
                    <p class="text-sm text-neutral-600 mb-2">"Vertical"</p>
                    <ButtonGroup orientation=ButtonGroupOrientation::Vertical>
                        <Button variant=ButtonVariant::Outline>"Top"</Button>
                        <Button variant=ButtonVariant::Outline>"Middle"</Button>
                        <Button variant=ButtonVariant::Outline>"Bottom"</Button>
                    </ButtonGroup>
                </div>
            </div>
        </div>
    }
}
