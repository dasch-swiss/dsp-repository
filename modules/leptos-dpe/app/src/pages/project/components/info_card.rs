use leptos::prelude::*;

#[component]
pub fn InfoCard(children: Children) -> impl IntoView {
    view! {
        <div class="bg-gray-50 border border-gray-200 rounded-lg p-3 text-gray-700 w-full">
            {children()}
        </div>
    }
}
