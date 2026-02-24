use leptos::prelude::*;

#[component]
pub fn PublicationYear(year: Option<String>) -> impl IntoView {
    view! {
        <div class="text-sm">
            <h3 class="font-semibold mb-2">"Data Publication Year"</h3>
            {year.map(|year| view! { <span>{year}</span> })}
        </div>
    }
}
