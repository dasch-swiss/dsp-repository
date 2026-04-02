use leptos::prelude::*;
use mosaic_tiles::icon::{Clock, Flag, Icon, LockClosed, LockOpen};

use crate::domain::{AccessRightsType, ProjectStatus};

#[component]
pub fn ProjectCardIndicators(status: ProjectStatus, access_rights: AccessRightsType) -> impl IntoView {
    let (status_icon, status_bg, status_text, status_label) = match status {
        ProjectStatus::Ongoing => (Clock, "bg-blue-100", "text-blue-600", "Ongoing"),
        ProjectStatus::Finished => (Flag, "bg-gray-100", "text-gray-600", "Finished"),
    };

    let (access_icon, access_bg, access_text, access_label) = match access_rights {
        AccessRightsType::FullOpenAccess => (LockOpen, "bg-green-100", "text-green-600", "Full Open Access"),
        AccessRightsType::OpenAccessWithRestrictions => {
            (LockOpen, "bg-yellow-100", "text-yellow-600", "Open Access with Restrictions")
        }
        AccessRightsType::EmbargoedAccess => (LockClosed, "bg-gray-100", "text-gray-600", "Embargoed Access"),
        AccessRightsType::MetadataOnlyAccess => (LockClosed, "bg-gray-100", "text-gray-600", "Metadata only Access"),
    };

    view! {
        <div class="absolute bottom-0 right-0 flex items-center gap-1.5 bg-gray-900/60 backdrop-blur-sm rounded-tl px-2 py-1.5">
            <div
                class=format!("tooltip {status_bg} {status_text} px-2.5 py-1.5 rounded")
                data-tip=status_label
            >
                <Icon icon=status_icon class="w-4 h-4" />
                <span class="sr-only">{status_label}</span>
            </div>
            <div
                class=format!("tooltip {access_bg} {access_text} px-2.5 py-1.5 rounded")
                data-tip=access_label
            >
                <Icon icon=access_icon class="w-4 h-4" />
                <span class="sr-only">{access_label}</span>
            </div>
        </div>
    }
}
