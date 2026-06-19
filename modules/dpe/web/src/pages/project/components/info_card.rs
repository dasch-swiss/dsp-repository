use maud::Markup;
use mosaic_tiles::card::{card, card_body, CardProps, CardVariant};

/// A muted, bordered card used to wrap small blocks of supplementary info
/// (permalink, citation, contributors, grants, …).
pub fn info_card(content: Markup) -> Markup {
    card(
        CardProps {
            variant: CardVariant::Bordered,
            class: "w-full bg-gray-50 text-gray-700 text-sm",
        },
        card_body("", content),
    )
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
