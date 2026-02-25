use leptos::prelude::*;

use super::contributor::Contributor;
use crate::domain::Attribution;

#[component]
pub fn AttributionsSection(attributions: Vec<Attribution>) -> impl IntoView {
    (!attributions.is_empty()).then(|| {
        view! {
            <div
                class="bg-base-100 p-6 rounded-lg scroll-mt-52"
            >
                <div class="grid md:grid-cols-1 lg:grid-cols-2 gap-2">
                    {attributions
                        .into_iter()
                        .map(|attr| view! { <Contributor attr /> })
                        .collect_view()}
                </div>
            </div>
        }
        .into_any()
    })
}
