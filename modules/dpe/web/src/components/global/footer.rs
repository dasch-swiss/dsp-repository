use maud::{html, Markup};
use mosaic_tiles::icon::{icon, Download, IconGitHub, IconLinkedIn, IconX};

fn current_year() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    1970 + secs / 31_557_600
}

pub fn footer() -> Markup {
    let year = current_year();

    html! {
        footer class="bg-slate-800 text-gray-300 py-12" {
            div class="dpe-max-layout-width mx-auto px-4" {
                nav class="flex flex-wrap justify-center gap-6 mb-8" {
                    a class="hover:text-white transition-colors"
                      href="https://dasch.swiss/legal-notice" target="_blank" rel="noopener noreferrer" {
                        "Legal Notice"
                    }
                    a class="hover:text-white transition-colors"
                      href="https://dasch.swiss/privacy-policy" target="_blank" rel="noopener noreferrer" {
                        "Privacy Policy"
                    }
                    a class="hover:text-white transition-colors"
                      href="https://dasch.swiss/privacy-policy-en" target="_blank" rel="noopener noreferrer" {
                        "Privacy Policy (English)"
                    }
                    a class="hover:text-white transition-colors"
                      href="https://dasch.swiss/impressum" target="_blank" rel="noopener noreferrer" {
                        "Impressum"
                    }
                    a class="hover:text-white transition-colors"
                      href="https://dasch.swiss/contact" target="_blank" rel="noopener noreferrer" {
                        "Contact"
                    }
                }

                hr class="border-slate-700 mb-8";

                div class="text-center mb-8" {
                    p class="tracking-wider uppercase text-white mb-4" { "Downloads" }
                    nav class="flex flex-wrap justify-center gap-8 text-sm" {
                        a class="hover:text-white transition-colors flex items-center gap-1 px-4"
                          href="https://dasch.swiss/downloads/AGB_DaSCH_4.0.pdf" target="_blank" rel="noopener noreferrer" {
                            (icon(Download, "w-4 h-4"))
                            "Terms and Conditions (AGB)"
                        }
                        a class="hover:text-white transition-colors flex items-center gap-1 px-4"
                          href="https://dasch.swiss/downloads/DaSCH_Deposit_Agreement.pdf" target="_blank" rel="noopener noreferrer" {
                            (icon(Download, "w-4 h-4"))
                            "Deposit Agreement"
                        }
                        a class="hover:text-white transition-colors flex items-center gap-1 px-4"
                          href="https://dasch.swiss/downloads/20220214_DaSCH_Statuten_Version_2022_def.pdf" target="_blank" rel="noopener noreferrer" {
                            (icon(Download, "w-4 h-4"))
                            "DaSCH Statutes 2022"
                        }
                        a class="hover:text-white transition-colors flex items-center gap-1 px-4"
                          href="https://dasch.swiss/downloads/ToS_NB_V07.pdf" target="_blank" rel="noopener noreferrer" {
                            (icon(Download, "w-4 h-4"))
                            "Terms of Service"
                        }
                    }
                }

                div class="flex justify-center gap-6 mb-8" {
                    a href="https://www.linkedin.com/company/dasch-swiss" target="_blank" rel="noopener noreferrer"
                      class="text-white hover:text-white transition-colors" aria-label="DaSCH on LinkedIn" {
                        (icon(IconLinkedIn, "w-6 h-6"))
                    }
                    a href="https://x.com/daschswiss" target="_blank" rel="noopener noreferrer"
                      class="text-white hover:text-white transition-colors" aria-label="DaSCH on X (Twitter)" {
                        (icon(IconX, "w-6 h-6"))
                    }
                    a href="https://github.com/dasch-swiss" target="_blank" rel="noopener noreferrer"
                      class="text-white hover:text-white transition-colors" aria-label="DaSCH on GitHub" {
                        (icon(IconGitHub, "w-6 h-6"))
                    }
                }

                div class="text-center text-sm text-gray-400" {
                    p {
                        (format!(
                            "© {year} DaSCH \u{2013} Swiss National Data and Service Center for the Humanities. All rights reserved."
                        ))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_footer_with_legal_and_download_links() {
        let out = footer().into_string();
        assert!(out.starts_with("<footer"), "{out}");
        assert!(out.contains(r#"href="https://dasch.swiss/legal-notice""#), "{out}");
        assert!(out.contains(r#"href="https://dasch.swiss/downloads/AGB_DaSCH_4.0.pdf""#), "{out}");
        assert!(out.contains("Deposit Agreement"), "{out}");
    }

    #[test]
    fn renders_social_links_with_aria_labels() {
        let out = footer().into_string();
        assert!(out.contains(r#"aria-label="DaSCH on LinkedIn""#), "{out}");
        assert!(out.contains(r#"aria-label="DaSCH on X (Twitter)""#), "{out}");
        assert!(out.contains(r#"href="https://github.com/dasch-swiss""#), "{out}");
    }

    #[test]
    fn renders_copyright_with_current_year() {
        let out = footer().into_string();
        let year = current_year();
        assert!(out.contains(&format!("© {year} DaSCH")), "{out}");
    }
}
