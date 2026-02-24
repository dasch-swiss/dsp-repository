use std::collections::HashMap;

use leptos::prelude::*;

use crate::components::project_detail::publications_section::PublicationsSection;
use crate::components::*;
use crate::domain::Publication;

#[component]
pub fn PublicationTab(abstracts: HashMap<String, AnyView>, publications: Option<Vec<Publication>>) -> impl IntoView {
    view! {
        <div id="abstract" class="scroll-mt-52">
            <LanguageTabs
                title="Abstract".to_string()
                content=abstracts
            />
        </div>

        {publications
            .map(|publications| {
                view! {
                    <PublicationsSection publications=publications />
                }
            })}
    }
}
