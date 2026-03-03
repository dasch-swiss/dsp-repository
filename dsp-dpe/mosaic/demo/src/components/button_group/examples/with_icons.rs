use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};
use mosaic_tiles::button_group::*;
use mosaic_tiles::icon::*;

#[component]
pub fn WithIconsExample() -> impl IntoView {
    view! {
        <div class="space-y-4">
            <div>
                <ButtonGroup>
                    <Button variant=ButtonVariant::Outline>
                        <Icon icon=IconChevronLeft class="w-4 h-4" />
                    </Button>
                    <Button variant=ButtonVariant::Outline>
                        <Icon icon=IconChevronRight class="w-4 h-4" />
                    </Button>
                </ButtonGroup>

            </div>
            <div>
                <ButtonGroup>
                    <Button variant=ButtonVariant::Outline>
                        <Icon icon=IconChevronLeft class="w-4 h-4 inline mr-2" />
                        "Prev"
                    </Button>
                    <Button variant=ButtonVariant::Outline>
                        "Next" <Icon icon=IconChevronRight class="w-4 h-4 inline ml-2" />
                    </Button>
                </ButtonGroup>

            </div>
        </div>
    }
}
