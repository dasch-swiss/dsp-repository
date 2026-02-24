use leptos::prelude::*;

#[component]
pub fn ProjectDates(start_date: String, end_date: String) -> impl IntoView {
    view! {
        <div class="text-sm">
            <span class="font-semibold">"Start Date: "</span>
            {start_date}
        </div>
        <div class="text-sm">
            <span class="font-semibold">"End Date: "</span>
            {end_date}
        </div>
    }
}
