use leptos::prelude::*;

use super::super::contributor::Contributor;
use crate::domain::ResolvedContributor;

#[component]
pub fn AttributionsSection(contributors: Vec<ResolvedContributor>) -> impl IntoView {
    (!contributors.is_empty()).then(|| {
        view! {
            <div class="grid md:grid-cols-1 lg:grid-cols-2 gap-2">
                {contributors
                    .into_iter()
                    .map(|c| view! { <Contributor contributor=c /> })
                    .collect_view()}
            </div>
        }
        .into_any()
    })
}
