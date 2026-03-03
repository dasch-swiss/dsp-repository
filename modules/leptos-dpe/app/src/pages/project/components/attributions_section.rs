use leptos::prelude::*;

use super::contributor::Contributor;
use crate::domain::Attribution;

#[component]
pub fn AttributionsSection(attributions: Vec<Attribution>) -> impl IntoView {
    (!attributions.is_empty()).then(|| {
        view! {
            <div class="grid md:grid-cols-1 lg:grid-cols-2 gap-2">
                {attributions.into_iter().map(|attr| view! { <Contributor attr /> }).collect_view()}
            </div>
        }
        .into_any()
    })
}
