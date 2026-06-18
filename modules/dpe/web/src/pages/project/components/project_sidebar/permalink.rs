use maud::{html, Markup};
use mosaic_tiles::copy_button::copy_button;

use super::super::info_card::info_card;

/// The project's permalink (PID) with a copy-to-clipboard button.
pub fn permalink(permalink: &str) -> Markup {
    html! {
        h3 class="dpe-subtitle" { "Permalink" }
        (info_card(html! {
            div class="flex items-center justify-between gap-3" {
                a href=(permalink) class="text-primary break-all flex-1" { (permalink) }
                (copy_button(permalink))
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_link_and_copy_button() {
        let out = permalink("https://ark.dasch.swiss/ark:/72163/1/0ABC").into_string();
        assert!(out.contains("Permalink"), "{out}");
        assert!(out.contains(r#"href="https://ark.dasch.swiss/ark:/72163/1/0ABC""#), "{out}");
        assert!(
            out.contains(r#"data-copy-text="https://ark.dasch.swiss/ark:/72163/1/0ABC""#),
            "copy button: {out}"
        );
    }
}
