use leptos::prelude::*;

#[component]
pub fn PublicationYear(year: String) -> impl IntoView {
    view! {
        <h3 class="dpe-subtitle">"Data Publication Year"</h3>
        <div>{year}</div>
    }
}
