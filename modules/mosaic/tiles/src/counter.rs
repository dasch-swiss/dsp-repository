use leptos::prelude::*;

#[island]
pub fn Counter() -> impl IntoView {
    let count = RwSignal::new(0);
    let on_click = move |_| *count.write() += 1;

    view! {
        <button on:click=on_click>"Click Me: " {count}</button>
    }
}
