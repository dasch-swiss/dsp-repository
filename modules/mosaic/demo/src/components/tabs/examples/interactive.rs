use leptos::prelude::*;
use mosaic_tiles::button::*;
use mosaic_tiles::tabs::*;

#[component]
pub fn InteractiveExample() -> impl IntoView {
    let (counter1, set_counter1) = signal(0);
    let (counter2, set_counter2) = signal(0);
    let (counter3, set_counter3) = signal(0);

    view! {
        <Tabs>
            <Tab
                name="interactive-tabs"
                value="counter1"
                label="Counter 1"
                checked=true
            >
                <div class="space-y-4">
                    <p class="text-2xl font-bold">"Count: " {counter1}</p>
                    <div class="flex gap-2">
                        <Button
                            variant=ButtonVariant::Primary
                            on_click=move |_| set_counter1.update(|n| *n += 1)
                        >
                            "Increment"
                        </Button>
                        <Button
                            variant=ButtonVariant::Outline
                            on_click=move |_| set_counter1.set(0)
                        >
                            "Reset"
                        </Button>
                    </div>
                </div>
            </Tab>
            <Tab
                name="interactive-tabs"
                value="counter2"
                label="Counter 2"
            >
                <div class="space-y-4">
                    <p class="text-2xl font-bold">"Count: " {counter2}</p>
                    <div class="flex gap-2">
                        <Button
                            variant=ButtonVariant::Primary
                            on_click=move |_| set_counter2.update(|n| *n += 1)
                        >
                            "Increment"
                        </Button>
                        <Button
                            variant=ButtonVariant::Outline
                            on_click=move |_| set_counter2.set(0)
                        >
                            "Reset"
                        </Button>
                    </div>
                </div>
            </Tab>
            <Tab
                name="interactive-tabs"
                value="counter3"
                label="Counter 3"
            >
                <div class="space-y-4">
                    <p class="text-2xl font-bold">"Count: " {counter3}</p>
                    <div class="flex gap-2">
                        <Button
                            variant=ButtonVariant::Primary
                            on_click=move |_| set_counter3.update(|n| *n += 1)
                        >
                            "Increment"
                        </Button>
                        <Button
                            variant=ButtonVariant::Outline
                            on_click=move |_| set_counter3.set(0)
                        >
                            "Reset"
                        </Button>
                    </div>
                </div>
            </Tab>
        </Tabs>
    }
}
