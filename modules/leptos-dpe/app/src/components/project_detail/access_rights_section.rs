use leptos::prelude::*;

use crate::domain::{AccessRights, AccessRightsType};

#[component]
pub fn AccessRightsSection(access_rights: AccessRights) -> impl IntoView {
    view! {
        <div
            id="access-rights"
            class="bg-base-100 p-6 rounded-lg scroll-mt-52"
        >
            <h3 class="text-xl font-bold mb-3">"Access Rights"</h3>
            <div class="space-y-2">
                <div class="badge badge-primary badge-lg">
                    {match access_rights.access_rights {
                        AccessRightsType::FullOpenAccess => "Full Open Access",
                        AccessRightsType::OpenAccessWithRestrictions => {
                            "Open Access with Restrictions"
                        }
                        AccessRightsType::EmbargoedAccess => "Embargoed Access",
                        AccessRightsType::MetadataOnlyAccess => {
                            "Metadata only Access"
                        }
                    }}
                </div>
                {access_rights
                    .embargo_date
                    .map(|date| {
                        view! {
                            <div class="text-sm">
                                "Embargo Date: " {date}
                            </div>
                        }
                    })}
            </div>
        </div>
    }
}
