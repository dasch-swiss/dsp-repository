use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonVariant};

#[component]
pub fn InteractiveExample() -> impl IntoView {
    let (count, set_count) = signal(0);
    let (disabled, set_disabled) = signal(false);

    view! {
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
    }
}
