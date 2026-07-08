use maud::{html, Markup};
use mosaic_tiles::card::{card, card_body, CardVariant};
use mosaic_tiles::link::link;
use mosaic_tiles::ComponentBuilder;

/// A bordered card with a bulleted list. When `as_links`, each item renders as a
/// link to itself; otherwise as plain text.
pub fn link_list_section(title: &str, items: &[String], as_links: bool) -> Markup {
    let body = html! {
        h3 class="text-base font-semibold mb-3" { (title) }
        ul class="list-disc list-inside text-sm" {
            @for item in items {
                @if as_links {
                    li { (link(item.as_str(), item.as_str())) }
                } @else {
                    li { (item) }
                }
            }
        }
    };
    card(card_body(body)).variant(CardVariant::Bordered).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_links_renders_anchors() {
        let items = vec!["https://example.org/doc".to_string()];
        let out = link_list_section("Documentation Material", &items, true).into_string();
        assert!(out.contains("Documentation Material"), "{out}");
        assert!(out.contains(r#"href="https://example.org/doc""#), "{out}");
    }

    #[test]
    fn plain_renders_list_text() {
        let items = vec!["Some material".to_string()];
        let out = link_list_section("Additional Material", &items, false).into_string();
        assert!(out.contains("<li>Some material</li>"), "{out}");
    }
}
