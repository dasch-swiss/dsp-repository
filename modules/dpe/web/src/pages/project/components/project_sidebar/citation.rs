use maud::{html, Markup};
use mosaic_tiles::copy_button::copy_button;

use super::super::info_card::info_card;

/// The "how to cite" string with a copy-to-clipboard button.
pub fn citation(citation: &str) -> Markup {
    html! {
        h3 class="dpe-subtitle" { "Citation" }
        ({
            info_card(
                html! {
                    div class = "flex items-center" { div class = "flex-1" { (citation) }
                    (copy_button(citation)) }
                },
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_citation_text_and_copy_button() {
        let out = citation("Author, A. (2024). Title.").into_string();
        assert!(out.contains("Citation"), "{out}");
        assert!(out.contains("Author, A. (2024). Title."), "{out}");
        assert!(out.contains(r#"data-copy-text="Author, A. (2024). Title.""#), "{out}");
    }
}
