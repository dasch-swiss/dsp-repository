use maud::{Markup, Render};
use mosaic_tiles::card::{card, card_body, CardVariant};
use mosaic_tiles::ComponentBuilder;

/// A muted, bordered card used to wrap small blocks of supplementary info
/// (permalink, citation, contributors, grants, …).
pub fn info_card(content: impl Render) -> Markup {
    card(card_body(content))
        .variant(CardVariant::Bordered)
        .class("w-full bg-gray-50 text-gray-700 text-sm")
        .build()
}

#[cfg(test)]
mod tests {
    use maud::html;

    use super::*;

    #[test]
    fn wraps_content_in_a_bordered_card() {
        let out = info_card(html! {
            "inner"
        })
        .into_string();
        assert!(out.contains("card card-bordered"), "{out}");
        assert!(out.contains("bg-gray-50"), "{out}");
        assert!(out.contains("inner"), "{out}");
    }
}
