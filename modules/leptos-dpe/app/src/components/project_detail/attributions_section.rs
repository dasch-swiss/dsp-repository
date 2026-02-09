use leptos::prelude::*;

use crate::components::Contributor;
use crate::domain::Attribution;

#[component]
pub fn AttributionsSection(attributions: Vec<Attribution>) -> impl IntoView {
    (!attributions.is_empty()).then(|| {
        view! {
            <div
                id="attributions"
                class="bg-base-100 p-6 rounded-lg scroll-mt-52"
            >
                <h3 class="text-xl font-bold mb-3">"Attributions"</h3>
                <div class="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
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
