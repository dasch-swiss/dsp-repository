use dpe_core::{AccessRightsType, ProjectStatus};
use maud::{html, Markup};
use mosaic_tiles::icon::{icon, Clock, Flag, LockClosed, LockOpen};

/// Status + access-rights indicator badges overlaid on a project card image.
pub fn project_card_indicators(status: ProjectStatus, access_rights: AccessRightsType) -> Markup {
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

    html! {
        div class="absolute bottom-0 right-0 flex items-center gap-1.5 bg-gray-900/60 backdrop-blur-sm rounded-tl px-2 py-1.5" {
            div class=(format!("tooltip {status_bg} {status_text} px-2.5 py-1.5 rounded")) data-tip=(status_label) {
                (icon(status_icon, "w-4 h-4"))
                span class="sr-only" { (status_label) }
            }
            div class=(format!("tooltip {access_bg} {access_text} px-2.5 py-1.5 rounded")) data-tip=(access_label) {
                (icon(access_icon, "w-4 h-4"))
                span class="sr-only" { (access_label) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ongoing_full_open_access_labels() {
        let out = project_card_indicators(ProjectStatus::Ongoing, AccessRightsType::FullOpenAccess).into_string();
        assert!(out.contains(r#"data-tip="Ongoing""#), "{out}");
        assert!(out.contains(r#"data-tip="Full Open Access""#), "{out}");
        assert!(out.contains("bg-blue-100"), "{out}");
        assert!(out.contains("bg-green-100"), "{out}");
    }

    #[test]
    fn finished_embargoed_labels() {
        let out = project_card_indicators(ProjectStatus::Finished, AccessRightsType::EmbargoedAccess).into_string();
        assert!(out.contains(r#"data-tip="Finished""#), "{out}");
        assert!(out.contains(r#"data-tip="Embargoed Access""#), "{out}");
    }

    #[test]
    fn includes_sr_only_text_for_each_badge() {
        let out = project_card_indicators(ProjectStatus::Ongoing, AccessRightsType::FullOpenAccess).into_string();
        assert_eq!(out.matches(r#"class="sr-only""#).count(), 2, "{out}");
    }
}
