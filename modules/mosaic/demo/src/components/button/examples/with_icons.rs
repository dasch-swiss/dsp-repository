use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};
use mosaic_tiles::icon::*;

#[component]
pub fn WithIconsExample() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div>
                <h4 class="text-base font-semibold mb-2">"Icon + Text"</h4>
                <div class="flex gap-4 items-center">
                    <Button variant=ButtonVariant::Primary>
                        <Icon icon=IconSearch class="w-4 h-4 inline mr-2" />
                        "Search"
                    </Button>
                    <Button variant=ButtonVariant::Secondary>
                        <Icon icon=Mail class="w-4 h-4 inline mr-2" />
                        "Send Email"
                    </Button>
                    <Button variant=ButtonVariant::Primary>
                        "Visit Docs" <Icon icon=LinkExternal class="w-4 h-4 inline ml-2" />
                    </Button>
                </div>
            </div>

            <div>
                <h4 class="text-base font-semibold mb-2">"Icon Only"</h4>
                <div class="flex gap-4 items-center">
                    <Button variant=ButtonVariant::Secondary>
                        <Icon icon=CopyPaste class="w-4 h-4" />
                    </Button>
                    <Button variant=ButtonVariant::Secondary>
                        <Icon icon=Info class="w-4 h-4" />
                    </Button>
                    <Button variant=ButtonVariant::Primary>
                        <Icon icon=IconChevronRight class="w-4 h-4" />
                    </Button>
                </div>
            </div>
        </div>
    }
}
