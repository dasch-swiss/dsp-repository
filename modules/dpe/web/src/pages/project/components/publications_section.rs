use dpe_core::Publication;
use maud::{html, Markup};
use mosaic_tiles::icon::{icon, Export};
use mosaic_tiles::link::{link, LinkProps};

use crate::pages::project::components::info_card::info_card;

/// A list of publications, each in an info card with optional PID link.
pub fn publications_section(publications: &[Publication]) -> Markup {
    html! {
        div {
            h3 class="dpe-subtitle" { "Publications" }
            div class="space-y-2 text-sm" {
                @for pub_ in publications {
                    (info_card(html! {
                        @if !pub_.text.is_empty() {
                            span { (pub_.text) " " }
                        }
                        @if let Some(pid) = &pub_.pid {
                            @let text = pid.text.clone().unwrap_or_else(|| pid.url.clone());
                            span class="ml-2" {
                                (link(LinkProps { href: &pid.url, ..Default::default() }, html! {
                                    (text) (icon(Export, "w-3 h-3"))
                                }))
                            }
                        }
                    }))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use dpe_core::project::Pid;

    use super::*;

    #[test]
    fn renders_publication_text_and_pid_link() {
        let pubs = vec![Publication {
            text: "A paper (2024)".to_string(),
            pid: Some(Pid {
                url: "https://doi.org/10.0/x".to_string(),
                text: Some("DOI".to_string()),
            }),
        }];
        let out = publications_section(&pubs).into_string();
        assert!(out.contains("Publications"), "{out}");
        assert!(out.contains("A paper (2024)"), "{out}");
        assert!(out.contains(r#"href="https://doi.org/10.0/x""#), "{out}");
        assert!(out.contains("DOI"), "{out}");
    }

    #[test]
    fn publication_without_pid_omits_link() {
        let pubs = vec![Publication { text: "No DOI here".to_string(), pid: None }];
        let out = publications_section(&pubs).into_string();
        assert!(out.contains("No DOI here"), "{out}");
        assert!(!out.contains("class=\"link"), "no link when pid absent: {out}");
    }
}
