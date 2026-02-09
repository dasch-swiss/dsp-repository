use leptos::prelude::*;

use crate::components::Person;
use crate::domain::Attribution;

#[component]
pub fn Contributor(attr: Attribution) -> impl IntoView {
    let person_id = attr.contributor.clone();
    view! {
        <div class="border-l-4 border-primary pl-4">
            <div class="text-xs text-base-content/60 mb-2">{attr.contributor_type.join(", ")}</div>
            <Person person_id=person_id />
        </div>
    }
}
