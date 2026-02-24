use leptos::prelude::*;

use crate::components::project_detail::publications_section::PublicationsSection;
use crate::domain::Publication;

#[component]
pub fn PublicationTab(abstract_en: Option<String>, publications: Option<Vec<Publication>>) -> impl IntoView {
    view! {
        <div class="space-y-4">
        <div>
            <h3 class="font-semibold mb-3">"Abstract"</h3>
            {abstract_en.map(|text| view! { <p class="leading-relaxed">{text}</p> })}
        </div>

        <div>
        {publications
            .map(|publications| {
                view! {
                    <PublicationsSection publications=publications />
                }
            })}
        </div>
        </div>
    }
}
