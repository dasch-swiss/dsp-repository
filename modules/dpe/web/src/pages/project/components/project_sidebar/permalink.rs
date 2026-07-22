use maud::{html, Markup};
use mosaic_tiles::copy_button::copy_button;

use super::super::info_card::info_card;

/// The project's permalink (PID) with a copy-to-clipboard button.
///
/// The ARK is shown as its bare identifier (`ark:/72163/1/0ABC`) — the way a
/// DOI is displayed without the `doi.org/` host — so the permalink stays short
/// and on one line. The link target and the copy button still use the full URL.
pub fn permalink(permalink: &str) -> Markup {
    let display = permalink
        .find("ark:/")
        .map(|i| &permalink[i..])
        .or_else(|| permalink.strip_prefix("https://"))
        .or_else(|| permalink.strip_prefix("http://"))
        .unwrap_or(permalink);
    let card_inner = html! {
        div class="flex items-center justify-between gap-3" {
            a href=(permalink) class="text-primary hover:underline break-all flex-1" { (display) }
            (copy_button(permalink))
        }
    };
    html! {
        h3 class="dpe-subtitle" { "Permalink" }
        (info_card(card_inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_link_and_copy_button() {
        let out = permalink("https://ark.dasch.swiss/ark:/72163/1/0ABC").into_string();
        assert!(out.contains("Permalink"), "{out}");
        // The link target and copy text keep the full, resolvable URL.
        assert!(out.contains(r#"href="https://ark.dasch.swiss/ark:/72163/1/0ABC""#), "{out}");
        assert!(
            out.contains(r#"data-copy-text="https://ark.dasch.swiss/ark:/72163/1/0ABC""#),
            "copy button: {out}"
        );
        // The visible link text is the bare ARK identifier (like a DOI, no host).
        assert!(out.contains(r#">ark:/72163/1/0ABC<"#), "display bare ark id: {out}");
        assert!(!out.contains(">ark.dasch.swiss"), "host stripped from display: {out}");
        // Underlines on hover, like the other text links (contributors, persons).
        assert!(out.contains("hover:underline"), "hover underline: {out}");
    }
}
