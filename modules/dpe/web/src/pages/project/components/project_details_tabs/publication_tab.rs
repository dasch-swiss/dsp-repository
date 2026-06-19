use dpe_core::Publication;
use maud::{html, Markup};

use crate::pages::project::components::publications_section::publications_section;

/// The "Publications" tab panel: an abstract (when present) and the
/// publications list (when present).
pub fn publication_tab(abstract_en: Option<&str>, publications: Option<&[Publication]>) -> Markup {
    html! {
        div class="space-y-4" {
            div {
                h3 class="dpe-subtitle" { "Abstract" }
                @if let Some(text) = abstract_en {
                    p class="text-sm text-gray-700" { (text) }
                }
            }
            div {
                @if let Some(pubs) = publications { (publications_section(pubs)) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use dpe_core::project::Pid;

    use super::*;

    #[test]
    fn renders_abstract_and_publications() {
        let pubs = vec![Publication {
            text: "A paper".to_string(),
            pid: Some(Pid { url: "https://doi.org/1".to_string(), text: None }),
        }];
        let out = publication_tab(Some("The abstract."), Some(&pubs)).into_string();
        assert!(out.contains("Abstract"), "{out}");
        assert!(out.contains("The abstract."), "{out}");
        assert!(out.contains("Publications"), "{out}");
        assert!(out.contains("A paper"), "{out}");
    }

    #[test]
    fn omits_abstract_paragraph_when_absent() {
        let out = publication_tab(None, None).into_string();
        assert!(out.contains("Abstract"), "heading still shows: {out}");
        assert!(!out.contains("text-sm text-gray-700"), "no abstract paragraph: {out}");
    }
}
