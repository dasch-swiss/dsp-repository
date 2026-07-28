use maud::{html, Markup};

use crate::components::global::header_links::header_links;

pub fn header() -> Markup {
    html! {
        div class="bg-white shadow-xs" {
            div class="flex items-center py-2 dpe-max-layout-width mx-auto px-4" {
                a href="/" aria-label="DaSCH Metadata Browser home" {
                    img src="/logo.svg" class="inline h-10 w-10 mr-2" alt="DaSCH logo";
                }

                div class="flex-1" {
                    a class="inline-flex items-center font-bold font-display text-xl" href="/" {
                        "DaSCH Metadata Browser"
                    }
                }
                div class="flex" { (header_links()) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_logo_and_home_links() {
        let out = header().into_string();
        assert!(out.contains(r#"<img src="/logo.svg""#), "{out}");
        assert!(out.contains(r#"aria-label="DaSCH Metadata Browser home""#), "{out}");
        assert!(out.contains("DaSCH Metadata Browser"), "{out}");
    }

    #[test]
    fn includes_header_links() {
        let out = header().into_string();
        // header_links renders the Help link to the about page.
        assert!(out.contains(r#"href="/dpe/about""#), "{out}");
    }
}
