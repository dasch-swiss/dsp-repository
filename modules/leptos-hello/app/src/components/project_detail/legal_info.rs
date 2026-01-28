use leptos::prelude::*;

use crate::domain::project::LegalInfo as LegalInfoData;

#[component]
pub fn LegalInfo(legal_info: Vec<LegalInfoData>) -> impl IntoView {
    legal_info
        .iter()
        .map(|info| {
            view! {
                <div class="space-y-2">
                    <div class="flex items-center gap-3 flex-wrap">
                        <span class="font-semibold">"License: "</span>
                        {match get_cc_license_info(
                            &info.license.license_uri,
                            &info.license.license_identifier,
                        ) {
                            Some((img_url, alt_text)) => {
                                view! {
                                    <a
                                        href=info.license.license_uri.clone()
                                        rel="noopener noreferrer"
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
                                view! {
                                    <a
                                        href=info.license.license_uri.clone()
                                        class="link link-primary"
                                    >
                                        {info.license.license_identifier.clone()}
                                    </a>
                                }
                                    .into_any()
                            }
                        }}
                    </div>
                    <div class="text-sm text-base-content/70">
                        "License Date: " {info.license.license_date.clone()}
                    </div>
                    <div class="text-sm">"Copyright Holder: " {info.copyright_holder.clone()}</div>
                    {(!info.authorship.is_empty())
                        .then(|| {
                            view! {
                                <div class="text-sm">
                                    "Authorship: " {info.authorship.join(", ")}
                                </div>
                            }
                        })}
                </div>
            }
        })
        .collect_view()
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
        format!(
            "Creative Commons {} 4.0 International License",
            license_name
        )
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
    } else if combined.contains("/by/") || combined.contains("cc by") || combined.contains("cc-by")
    {
        Some("by".to_string())
    } else if combined.contains("cc0") || combined.contains("publicdomain/zero") {
        Some("cc0".to_string())
    } else {
        None
    }
}
