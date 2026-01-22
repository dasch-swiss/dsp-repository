use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonType, ButtonVariant};

#[component]
pub fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    let (disabled, set_disabled) = signal(false);
    view! {
        <Button disabled=disabled on_click=move |_| set_count.update(|n| *n += 1)>
            "Click Me: "
            {count}
        </Button>
        <div>"Disabled: " {move || disabled.get()}</div>
        <Button on_click=move |_| set_disabled.update(|b| *b = !*b)>"Disable other button"</Button>

        <Button variant=ButtonVariant::Secondary on_click=move |_| set_count.set(0)>
            "Reset counter"
        </Button>

        <Button button_type=ButtonType::Submit>"Submit"</Button>
        <Button button_type=ButtonType::Reset>"Reset"</Button>
        <Button button_type=ButtonType::Button>"Button"</Button>
    }
}
