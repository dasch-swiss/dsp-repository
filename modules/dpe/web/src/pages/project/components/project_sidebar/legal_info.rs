use leptos::prelude::*;
use mosaic_tiles::link::Link;

use super::super::info_card::InfoCard;
use super::super::person::{AffiliationName, Person};
use crate::components::{should_render_value, PlaceholderValue};
use crate::domain::LegalInfo as LegalInfoData;

// See `crate::pages::project::components` module docs for why this is a sync
// lookup with a wasm32 stub.

/// Renders an entity name (person or organization) from the in-process caches.
#[cfg(not(target_arch = "wasm32"))]
#[component]
fn EntityName(id: String) -> impl IntoView {
    if id.starts_with("person-") || id.contains("-person-") {
        let name = dpe_core::load_person(&id)
            .map(|p| format!("{} {}", p.given_names.join(" "), p.family_names.join(" ")))
            .unwrap_or_else(|| id.clone());
        view! { <span>{name}</span> }.into_any()
    } else if id.starts_with("organization-") || id.contains("-organization-") {
        let name = dpe_core::load_organization(&id).map(|o| o.name).unwrap_or_else(|| id.clone());
        view! { <span>{name}</span> }.into_any()
    } else {
        view! { <span>{id}</span> }.into_any()
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
fn EntityName(id: String) -> impl IntoView {
    let _ = id;
}

#[component]
pub fn LegalInfo(legal_info: Vec<LegalInfoData>) -> impl IntoView {
    legal_info
        .iter()
        .map(|info| {
            let license_is_placeholder = dpe_core::is_placeholder(&info.license.license_identifier)
                || dpe_core::is_placeholder(&info.license.license_uri);

            view! {
                {if license_is_placeholder {
                    should_render_value(&info.license.license_identifier)
                        .then(|| {
                            // Placeholder: hide in production, show red in dev/stage
                            view! {
                                <div>
                                    <div class="dpe-subtitle">"License"</div>
                                    <PlaceholderValue value=info
                                        .license
                                        .license_identifier
                                        .clone() />
                                </div>
                            }
                        })
                        .into_any()
                } else {
                    view! {
                        <div>
                            <div class="dpe-subtitle">"License"</div>
                            {match get_cc_license_info(
                                &info.license.license_uri,
                                &info.license.license_identifier,
                            ) {
                                Some((img_url, alt_text)) => {
                                    view! {
                                        <a
                                            href=info.license.license_uri.clone()
                                            rel="noopener noreferrer"
                                            class="block mb-1"
                                        >
                                            <img
                                                src=img_url
                                                alt=alt_text
                                                class="h-8"
                                                title=info.license.license_identifier.clone()
                                            />
                                        </a>
                                    }
                                        .into_any()
                                }
                                None => {
                                    let href = info.license.license_uri.clone();
                                    let text = info.license.license_identifier.clone();
                                    view! { <Link href=href>{text}</Link> }.into_any()
                                }
                            }}
                            <div>"(" {info.license.license_date.clone()} ")"</div>
                        </div>
                    }
                        .into_any()
                }}

                {if dpe_core::is_placeholder(&info.copyright_holder) {
                    should_render_value(&info.copyright_holder)
                        .then(|| {
                            view! {
                                <div>
                                    <h3 class="dpe-subtitle">"Copyright"</h3>
                                    <PlaceholderValue value=info.copyright_holder.clone() />
                                </div>
                            }
                        })
                        .into_any()
                } else {
                    view! {
                        <div>
                            <h3 class="dpe-subtitle">"Copyright"</h3>
                            <EntityName id=info.copyright_holder.clone() />
                        </div>
                    }
                        .into_any()
                }}
                <div>
                    {
                        let ids: Vec<String> = info
                            .authorship
                            .iter()
                            .filter(|id| should_render_value(id))
                            .cloned()
                            .collect();
                        (!ids.is_empty())
                            .then(|| {
                                view! {
                                    <div class="dpe-subtitle">"Authorship"</div>
                                    <div>
                                        {ids
                                            .into_iter()
                                            .map(|id| {
                                                if dpe_core::is_placeholder(&id) {
                                                    view! {
                                                        <div>
                                                            <PlaceholderValue value=id />
                                                        </div>
                                                    }
                                                        .into_any()
                                                } else {
                                                    view! {
                                                        <div>
                                                            <EntityName id=id />
                                                        </div>
                                                    }
                                                        .into_any()
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                }
                            })
                    }
                </div>
            }
        })
        .collect_view()
}

#[component]
pub fn ContactSection(ids: Vec<String>) -> impl IntoView {
    view! {
        <h3 class="dpe-subtitle">"Contact"</h3>
        <div class="space-y-2">
            {ids
                .into_iter()
                .map(|id| {
                    if id.starts_with("organization-") || id.contains("-organization-") {
                        view! {
                            <InfoCard>
                                <AffiliationName org_id=id />
                            </InfoCard>
                        }
                            .into_any()
                    } else {
                        view! {
                            <InfoCard>
                                <Person person_id=id roles=None show_email=true />
                            </InfoCard>
                        }
                            .into_any()
                    }
                })
                .collect_view()}
        </div>
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
