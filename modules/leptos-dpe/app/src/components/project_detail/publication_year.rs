use leptos::prelude::*;

#[component]
pub fn PublicationYear(year: Option<String>) -> impl IntoView {
    year.map(|year| {
        view! {
            <div class="text-sm">
                <span class="font-semibold">"Publication Year: "</span>
                {year}
            </div>
        }
    })
}
