use leptos::prelude::*;

use crate::components::{InfoCard, Person};
use crate::domain::Attribution;

#[component]
pub fn Contributor(attr: Attribution) -> impl IntoView {
    let person_id = attr.contributor.clone();
    view! {
        <InfoCard>
            <div class="text-xs text-base-content/60 mb-2">{attr.contributor_type.join(", ")}</div>
            <Person person_id=person_id />
        </InfoCard>
    }
}
