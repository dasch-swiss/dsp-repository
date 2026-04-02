use leptos::prelude::*;
use mosaic_tiles::button::{Button, ButtonType};

#[component]
pub fn TypesExample() -> impl IntoView {
    view! {
        <div class="flex gap-4 items-center">
            <Button button_type=ButtonType::Button>"Button"</Button>
            <Button button_type=ButtonType::Submit>"Submit"</Button>
            <Button button_type=ButtonType::Reset>"Reset"</Button>
        </div>
    }
}
