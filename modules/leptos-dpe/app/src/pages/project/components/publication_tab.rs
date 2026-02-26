use leptos::prelude::*;

use crate::domain::Publication;
use crate::pages::project::components::publications_section::PublicationsSection;

#[component]
pub fn PublicationTab(abstract_en: Option<String>, publications: Option<Vec<Publication>>) -> impl IntoView {
    view! {
        <div class="space-y-4">
        <div>
            <h3 class="dpe-subtitle">"Abstract"</h3>
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
