use leptos::prelude::*;

use crate::domain::{AccessRights, AccessRightsType};

#[component]
pub fn AccessRightsSection(access_rights: AccessRights) -> impl IntoView {
    view! {
        <div>
            <div class="dpe-subtitle">"Access Rights"</div>
            <div>
                {match access_rights.access_rights {
                    AccessRightsType::FullOpenAccess => "Full Open Access",
                    AccessRightsType::OpenAccessWithRestrictions => "Open Access with Restrictions",
                    AccessRightsType::EmbargoedAccess => "Embargoed Access",
                    AccessRightsType::MetadataOnlyAccess => "Metadata only Access",
                }}
            </div>
            {access_rights
                .embargo_date
                .map(|date| {
                    view! { <div>"Embargo Date: " {date}</div> }
                })}
        </div>
    }
}
