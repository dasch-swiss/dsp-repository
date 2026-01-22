use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonType, ButtonVariant};
use mosaic_tiles::icon::*;

#[component]
pub fn ButtonExamples() -> impl IntoView {
    let (count, set_count) = signal(0);
    let (disabled, set_disabled) = signal(false);

    view! {
        <div class="space-y-8">
            <section>
                <h2 class="text-2xl font-bold mb-4">"Button Component Examples"</h2>

                <div class="space-y-6">
                    <div>
                        <h3 class="text-lg font-semibold mb-3">"Variants"</h3>
                        <div class="flex gap-4 items-center">
                            <Button variant=ButtonVariant::Primary>"Primary Button"</Button>
                            <Button variant=ButtonVariant::Secondary>"Secondary Button"</Button>
                        </div>
                    </div>

                    <div>
                        <h3 class="text-lg font-semibold mb-3">"Button Types"</h3>
                        <div class="flex gap-4 items-center">
                            <Button button_type=ButtonType::Button>"Button"</Button>
                            <Button button_type=ButtonType::Submit>"Submit"</Button>
                            <Button button_type=ButtonType::Reset>"Reset"</Button>
                        </div>
                    </div>

                    <div>
                        <h3 class="text-lg font-semibold mb-3">"Disabled State"</h3>
                        <div class="flex gap-4 items-center">
                            <Button disabled=true variant=ButtonVariant::Primary>
                                "Disabled Primary"
                            </Button>
                            <Button disabled=true variant=ButtonVariant::Secondary>
                                "Disabled Secondary"
                            </Button>
                        </div>
                    </div>
                </div>
            </section>

            <section>
                <h3 class="text-xl font-bold mb-4">"Interactive Counter Example"</h3>

                <div class="space-y-4">
                    <div class="flex gap-4 items-center">
                        <Button disabled=disabled on_click=move |_| set_count.update(|n| *n += 1)>
                            "Increment: "
                            {count}
                        </Button>
                        <Button variant=ButtonVariant::Secondary on_click=move |_| set_count.set(0)>
                            "Reset"
                        </Button>
                    </div>

                    <div class="flex gap-4 items-center">
                        <span class="text-gray-700">"Counter: " {count}</span>
                        <span class="text-gray-700">
                            "Button disabled: " {move || disabled.get().to_string()}
                        </span>
                    </div>

                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=move |_| set_disabled.update(|b| *b = !*b)
                    >
                        {move || if disabled.get() { "Enable" } else { "Disable" }}
                        " increment button"
                    </Button>
                </div>
            </section>

            <section>
                <h3 class="text-xl font-bold mb-4">"Buttons with Icons"</h3>

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
                                "Visit Docs"
                                <Icon icon=LinkExternal class="w-4 h-4 inline ml-2" />
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
            </section>

        </div>
    }
}
