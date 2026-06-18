use maud::{html, Markup};

/// A list of card-style entries — each `(href, name, description)`. When
/// `clickable`, each card is a link; otherwise a plain card. Empty → nothing.
pub fn link_card_section(title: &str, items: &[(String, String, String)], clickable: bool) -> Markup {
    if items.is_empty() {
        return html! {};
    }
    html! {
        div {
            h3 class="dpe-subtitle" { (title) }
            div class="flex flex-col gap-2" {
                @for (href, name, description) in items {
                    @if clickable {
                        a href=(href) class="block bg-gray-50 border border-gray-200 rounded p-3 hover:border-primary-400 transition-colors" {
                            div class="font-medium text-gray-900" { (name) }
                            div class="text-sm text-gray-600 line-clamp-2" { (description) }
                        }
                    } @else {
                        div class="block bg-gray-50 border border-gray-200 rounded p-3" {
                            div class="font-medium text-gray-900" { (name) }
                            div class="text-sm text-gray-600 line-clamp-2" { (description) }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<(String, String, String)> {
        vec![("/cluster/1".to_string(), "Cluster One".to_string(), "A cluster".to_string())]
    }

    #[test]
    fn clickable_renders_links() {
        let out = link_card_section("Part of Cluster", &items(), true).into_string();
        assert!(out.contains(r#"<a href="/cluster/1""#), "{out}");
        assert!(out.contains("Cluster One"), "{out}");
    }

    #[test]
    fn non_clickable_renders_plain_cards() {
        let out = link_card_section("Part of Cluster", &items(), false).into_string();
        assert!(!out.contains("<a "), "non-clickable has no links: {out}");
        assert!(out.contains("Cluster One"), "{out}");
    }

    #[test]
    fn empty_renders_nothing() {
        assert_eq!(link_card_section("X", &[], true).into_string(), "");
    }
}
