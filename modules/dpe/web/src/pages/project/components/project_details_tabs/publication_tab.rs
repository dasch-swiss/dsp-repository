use dpe_core::Publication;
use maud::{html, Markup};

use crate::pages::project::components::publications_section::publications_section;

/// The "Publications" tab panel: the publications list (when present).
pub fn publication_tab(publications: Option<&[Publication]>) -> Markup {
    html! {
        div class="space-y-4" {
            @if let Some(pubs) = publications { (publications_section(pubs)) }
        }
    }
}

#[cfg(test)]
mod tests {
    use dpe_core::project::Pid;

    use super::*;

    #[test]
    fn renders_publications() {
        let pubs = vec![Publication {
            text: "A paper".to_string(),
            pid: Some(Pid { url: "https://doi.org/1".to_string(), text: None }),
        }];
        let out = publication_tab(Some(&pubs)).into_string();
        assert!(out.contains("Publications"), "{out}");
        assert!(out.contains("A paper"), "{out}");
    }

    #[test]
    fn renders_empty_panel_when_no_publications() {
        let out = publication_tab(None).into_string();
        assert!(!out.contains("A paper"), "{out}");
        assert!(!out.contains("Abstract"), "abstract no longer lives here: {out}");
    }
}
