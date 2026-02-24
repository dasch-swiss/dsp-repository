use leptos::prelude::*;

use crate::components::project_detail::publications_section::PublicationsSection;
use crate::domain::Publication;

#[component]
pub fn PublicationTab(abstract_en: Option<String>, publications: Option<Vec<Publication>>) -> impl IntoView {
    view! {
        <div id="abstract" class="scroll-mt-52">
            <h3 class="text-xl font-bold mb-3">"Abstract"</h3>
            {abstract_en.map(|text| view! { <p class="leading-relaxed">{text}</p> })}
        </div>

        {publications
            .map(|publications| {
                view! {
                    <PublicationsSection publications=publications />
                }
            })}
    }
}
