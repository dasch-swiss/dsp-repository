use leptos::prelude::*;

const TRUNCATE_THRESHOLD: usize = 500;

#[island]
pub fn Description(text: String) -> impl IntoView {
    let is_long = text.len() > TRUNCATE_THRESHOLD;
    let (expanded, set_expanded) = signal(false);

    view! {
        <p class="text-lg text-gray-600" class:line-clamp-4=move || is_long && !expanded.get()>
            {text}
        </p>
        {is_long
            .then(|| {
                view! {
                    <button
                        class="font-semibold text-primary cursor-pointer mt-2"
                        on:click=move |_| set_expanded.update(|v| *v = !*v)
                    >
                        {move || if expanded.get() { "Show less" } else { "Show more" }}
                    </button>
                }
            })}
    }
}
