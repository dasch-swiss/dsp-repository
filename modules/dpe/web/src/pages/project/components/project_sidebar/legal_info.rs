use dpe_core::project::LegalInfo as LegalInfoData;
use maud::{html, Markup};
use mosaic_tiles::link::{link, LinkProps};

use super::super::info_card::info_card;
use super::super::person::{affiliation_name, person};
use crate::components::{placeholder_value, should_render_value};

/// Render an entity name (person or organization) from the in-process caches,
/// by ID. Falls back to the raw ID when not found or not an entity reference.
fn entity_name(id: &str) -> Markup {
    if id.starts_with("person-") || id.contains("-person-") {
        let name = dpe_core::load_person(id)
            .map(|p| format!("{} {}", p.given_names.join(" "), p.family_names.join(" ")))
            .unwrap_or_else(|| id.to_string());
        html! { span { (name) } }
    } else if id.starts_with("organization-") || id.contains("-organization-") {
        let name = dpe_core::load_organization(id)
            .map(|o| o.name)
            .unwrap_or_else(|| id.to_string());
        html! { span { (name) } }
    } else {
        html! { span { (id) } }
    }
}

/// The legal-info block: license (CC badge or link), copyright holder, and
/// authorship. Placeholder values are hidden in production, shown red in dev.
pub fn legal_info(legal_info: &[LegalInfoData]) -> Markup {
    html! {
        @for info in legal_info {
            @let license_is_placeholder = dpe_core::is_placeholder(&info.license.license_identifier)
                || dpe_core::is_placeholder(&info.license.license_uri);
            @if license_is_placeholder {
                @if should_render_value(&info.license.license_identifier) {
                    div {
                        div class="dpe-subtitle" { "License" }
                        (placeholder_value(&info.license.license_identifier))
                    }
                }
            } @else {
                div {
                    div class="dpe-subtitle" { "License" }
                    @match get_cc_license_info(&info.license.license_uri, &info.license.license_identifier) {
                        Some((img_url, alt_text)) => {
                            a href=(info.license.license_uri) rel="noopener noreferrer" class="block mb-1" {
                                img src=(img_url) alt=(alt_text) class="h-8" title=(info.license.license_identifier);
                            }
                        }
                        None => {
                            (link(
                                LinkProps { href: &info.license.license_uri, ..Default::default() },
                                html! { (info.license.license_identifier) },
                            ))
                        }
                    }
                    div { "(" (info.license.license_date) ")" }
                }
            }

            @if dpe_core::is_placeholder(&info.copyright_holder) {
                @if should_render_value(&info.copyright_holder) {
                    div {
                        h3 class="dpe-subtitle" { "Copyright" }
                        (placeholder_value(&info.copyright_holder))
                    }
                }
            } @else {
                div {
                    h3 class="dpe-subtitle" { "Copyright" }
                    (entity_name(&info.copyright_holder))
                }
            }

            div {
                @let ids: Vec<&str> = info
                    .authorship
                    .iter()
                    .map(String::as_str)
                    .filter(|&id| should_render_value(id))
                    .collect();
                @if !ids.is_empty() {
                    div class="dpe-subtitle" { "Authorship" }
                    div {
                        @for id in ids {
                            @if dpe_core::is_placeholder(id) {
                                div { (placeholder_value(id)) }
                            } @else {
                                div { (entity_name(id)) }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The contact block: each contact is an org affiliation name or a person card
/// (with email).
pub fn contact_section(ids: &[String]) -> Markup {
    html! {
        h3 class="dpe-subtitle" { "Contact" }
        div class="space-y-2" {
            @for id in ids {
                @if id.starts_with("organization-") || id.contains("-organization-") {
                    (info_card(affiliation_name(id)))
                } @else {
                    (info_card(person(id, None, true)))
                }
            }
        }
    }
}

fn get_cc_license_info(license_uri: &str, license_identifier: &str) -> Option<(String, String)> {
    let license_lower = license_uri.to_lowercase();
    let identifier_lower = license_identifier.to_lowercase();

    if !license_lower.contains("creativecommons.org") && !identifier_lower.contains("cc ") {
        return None;
    }

    let license_type = extract_cc_license_type(&license_lower, &identifier_lower)?;

    let img_url = if license_type == "cc0" {
        "/assets/images/cc-licenses/cc0-1.0.svg".to_string()
    } else {
        format!("/assets/images/cc-licenses/{}-4.0.svg", license_type)
    };

    let alt_text = generate_cc_alt_text(&license_type);

    Some((img_url, alt_text))
}

fn generate_cc_alt_text(license_type: &str) -> String {
    let license_name = match license_type {
        "by" => "Attribution",
        "by-sa" => "Attribution-ShareAlike",
        "by-nd" => "Attribution-NoDerivatives",
        "by-nc" => "Attribution-NonCommercial",
        "by-nc-sa" => "Attribution-NonCommercial-ShareAlike",
        "by-nc-nd" => "Attribution-NonCommercial-NoDerivatives",
        "cc0" => "Public Domain Dedication",
        _ => "License",
    };

    if license_type == "cc0" {
        "CC0 1.0 Public Domain Dedication".to_string()
    } else {
        format!("Creative Commons {} 4.0 International License", license_name)
    }
}

fn extract_cc_license_type(license_uri: &str, license_identifier: &str) -> Option<String> {
    let combined = format!("{} {}", license_uri, license_identifier).to_lowercase();

    if combined.contains("by-nc-nd") || combined.contains("by_nc_nd") {
        Some("by-nc-nd".to_string())
    } else if combined.contains("by-nc-sa") || combined.contains("by_nc_sa") {
        Some("by-nc-sa".to_string())
    } else if combined.contains("by-nd") || combined.contains("by_nd") {
        Some("by-nd".to_string())
    } else if combined.contains("by-sa") || combined.contains("by_sa") {
        Some("by-sa".to_string())
    } else if combined.contains("by-nc") || combined.contains("by_nc") {
        Some("by-nc".to_string())
    } else if combined.contains("/by/") || combined.contains("cc by") || combined.contains("cc-by") {
        Some("by".to_string())
    } else if combined.contains("cc0") || combined.contains("publicdomain/zero") {
        Some("cc0".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use dpe_core::project::License;

    use super::*;

    fn legal(identifier: &str, uri: &str, copyright: &str) -> LegalInfoData {
        LegalInfoData {
            license: License {
                license_identifier: identifier.to_string(),
                license_date: "2024-01-01".to_string(),
                license_uri: uri.to_string(),
            },
            copyright_holder: copyright.to_string(),
            authorship: vec!["Free Text Author".to_string()],
        }
    }

    #[test]
    fn cc_license_renders_badge_image() {
        let info = vec![legal(
            "CC BY 4.0",
            "https://creativecommons.org/licenses/by/4.0/",
            "DaSCH",
        )];
        let out = legal_info(&info).into_string();
        assert!(out.contains("/assets/images/cc-licenses/by-4.0.svg"), "{out}");
        assert!(out.contains("(2024-01-01)"), "license date: {out}");
    }

    #[test]
    fn non_cc_license_renders_link() {
        let info = vec![legal("MIT", "https://opensource.org/license/mit", "DaSCH")];
        let out = legal_info(&info).into_string();
        assert!(out.contains(r#"href="https://opensource.org/license/mit""#), "{out}");
        assert!(out.contains("MIT"), "{out}");
    }

    #[test]
    fn free_text_copyright_and_authorship_render() {
        let info = vec![legal(
            "CC BY 4.0",
            "https://creativecommons.org/licenses/by/4.0/",
            "ACME Corp",
        )];
        let out = legal_info(&info).into_string();
        assert!(out.contains("Copyright"), "{out}");
        assert!(out.contains("ACME Corp"), "{out}");
        assert!(out.contains("Authorship"), "{out}");
        assert!(out.contains("Free Text Author"), "{out}");
    }

    #[test]
    fn cc_license_type_extraction() {
        assert_eq!(
            extract_cc_license_type("creativecommons.org/licenses/by-nc-nd/4.0", ""),
            Some("by-nc-nd".to_string())
        );
        assert_eq!(
            extract_cc_license_type("creativecommons.org/publicdomain/zero/1.0", ""),
            Some("cc0".to_string())
        );
        assert_eq!(
            extract_cc_license_type("creativecommons.org/licenses/by/4.0", ""),
            Some("by".to_string())
        );
    }
}
