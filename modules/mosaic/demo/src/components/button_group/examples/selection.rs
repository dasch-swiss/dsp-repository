use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};
use mosaic_tiles::button_group::*;

#[component]
pub fn SelectionExample() -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <div class="space-y-4">
            <p class="text-sm text-gray-600">"Count: " {move || count.get()}</p>

            <ButtonGroup>
                <Button
                    variant=ButtonVariant::Outline
                    on_click=move |_| set_count.update(|n| *n -= 1)
                >
                    "-"
                </Button>
                <Button variant=ButtonVariant::Outline on_click=move |_| set_count.set(0)>
                    "Reset"
                </Button>
                <Button
                    variant=ButtonVariant::Outline
                    on_click=move |_| set_count.update(|n| *n += 1)
                >
                    "+"
                </Button>
            </ButtonGroup>
        </div>
    }
}
