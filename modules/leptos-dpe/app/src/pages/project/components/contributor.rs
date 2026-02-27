use leptos::prelude::*;

use super::info_card::InfoCard;
use super::person::Person;
use crate::domain::Attribution;

#[component]
pub fn Contributor(attr: Attribution) -> impl IntoView {
    let person_id = attr.contributor.clone();
    view! {
        <InfoCard>
            <div class="text-sm">
                <Person person_id=person_id />
                <div>{attr.contributor_type.join(", ")}</div>
            </div>
        </InfoCard>
    }
}
