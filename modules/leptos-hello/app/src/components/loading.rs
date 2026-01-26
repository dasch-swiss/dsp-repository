use leptos::prelude::*;

#[component]
pub fn Loading() -> impl IntoView {
    view! {
        <div class="flex items-center justify-center w-100 h-100">
            <span class="loading loading-spinner loading-xl"></span>
        </div>
    }
}
