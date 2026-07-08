use dpe_core::project::{AccessRights, AccessRightsType};
use maud::{html, Markup};

/// The data-access block: access-rights label and (if present) embargo date.
pub fn access_rights_section(access_rights: &AccessRights) -> Markup {
    html! {
        div {
            div class="dpe-subtitle" { "Access Rights" }
            div {
                ({
                    match access_rights.access_rights {
                        AccessRightsType::FullOpenAccess => "Full Open Access",
                        AccessRightsType::OpenAccessWithRestrictions => {
                            "Open Access with Restrictions"
                        }
                        AccessRightsType::EmbargoedAccess => "Embargoed Access",
                        AccessRightsType::MetadataOnlyAccess => "Metadata only Access",
                    }
                })
            }
            @if let Some(date) = &access_rights.embargo_date {
                div { "Embargo Date: " (date) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_label_and_embargo() {
        let ar = AccessRights {
            access_rights: AccessRightsType::EmbargoedAccess,
            embargo_date: Some("2030-01-01".to_string()),
        };
        let out = access_rights_section(&ar).into_string();
        assert!(out.contains("Embargoed Access"), "{out}");
        assert!(out.contains("Embargo Date: 2030-01-01"), "{out}");
    }

    #[test]
    fn omits_embargo_when_absent() {
        let ar = AccessRights {
            access_rights: AccessRightsType::FullOpenAccess,
            embargo_date: None,
        };
        let out = access_rights_section(&ar).into_string();
        assert!(out.contains("Full Open Access"), "{out}");
        assert!(!out.contains("Embargo Date"), "{out}");
    }
}
