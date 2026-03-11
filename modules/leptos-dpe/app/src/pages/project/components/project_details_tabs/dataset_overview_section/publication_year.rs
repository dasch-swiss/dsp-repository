use leptos::prelude::*;

#[component]
pub fn PublicationYear(year: Option<String>) -> impl IntoView {
    view! {
            <h3 class="dpe-subtitle">"Data Publication Year"</h3>
            {year.map(|year| view! { <span>{year}</span> })}
    }
}
