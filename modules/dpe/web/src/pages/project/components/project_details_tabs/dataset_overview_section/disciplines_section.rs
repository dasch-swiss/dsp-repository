use dpe_core::project::Discipline;
use maud::{html, Markup};

const CHIP: &str = "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-primary-50 text-primary-700";

/// "Disciplines" chips. Reference disciplines link out; text disciplines render
/// as plain chips. Renders nothing when empty.
pub fn disciplines_section(disciplines: &[Discipline]) -> Markup {
    if disciplines.is_empty() {
        return html! {};
    }
    html! {
        div {
            h3 class="dpe-subtitle" { "Disciplines" }
            div class="flex flex-wrap gap-1.5" {
                @for d in disciplines {
                    @let (label, url) = match d {
                        Discipline::Text(map) => (dpe_core::lang_value(map).cloned().unwrap_or_default(), None),
                        Discipline::Reference(r) => (r.text.clone().unwrap_or_else(|| r.url.clone()), Some(r.url.clone())),
                    };
                    @match url {
                        Some(href) => a href=(href) { span class=(CHIP) { (label) } },
                        None => span class=(CHIP) { (label) },
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use dpe_core::models::AuthorityFileReference;

    use super::*;

    #[test]
    fn text_discipline_is_a_plain_chip() {
        let d = vec![Discipline::Text(HashMap::from([(
            "en".to_string(),
            "History".to_string(),
        )]))];
        let out = disciplines_section(&d).into_string();
        assert!(out.contains("Disciplines"), "{out}");
        assert!(out.contains(">History</span>"), "{out}");
        assert!(!out.contains("<a "), "text discipline has no link: {out}");
    }

    #[test]
    fn reference_discipline_links_out() {
        let d = vec![Discipline::Reference(AuthorityFileReference {
            type_: "URL".to_string(),
            url: "https://skos.um.es/d/1".to_string(),
            text: Some("Archaeology".to_string()),
        })];
        let out = disciplines_section(&d).into_string();
        assert!(out.contains(r#"<a href="https://skos.um.es/d/1">"#), "{out}");
        assert!(out.contains("Archaeology"), "{out}");
    }

    #[test]
    fn empty_renders_nothing() {
        assert_eq!(disciplines_section(&[]).into_string(), "");
    }
}
