use maud::{html, Markup};

/// Legal links every DaSCH service carries.
///
/// DPE's footer additionally carries downloads, social links and marketing
/// copy; none of that belongs on an authenticated tool used by about thirty
/// people who reached it deliberately, so this is the legal minimum.
const LEGAL_LINKS: [(&str, &str); 3] = [
    ("Legal Notice", "https://dasch.swiss/legal-notice"),
    ("Privacy Policy", "https://dasch.swiss/privacy-policy"),
    ("Impressum", "https://dasch.swiss/impressum"),
];

/// The global footer: a slim row of legal links.
///
/// Plain anchors inheriting the footer's `text-gray-300`, matching DPE's footer,
/// rather than the Mosaic `link` tile. The tile hardcodes `text-primary-600` for
/// light surfaces, which measures 2.35:1 against `bg-slate-800` — below WCAG 2.1
/// AA's 4.5:1 for text this size. The inherited grey is 9.93:1. A dark-surface
/// variant of the tile would be the other fix, but that is a design-system
/// decision, not something a footer should settle on its own.
pub fn footer() -> Markup {
    html! {
        footer class="bg-slate-800 text-gray-300 py-6" {
            nav class="flex flex-wrap justify-center gap-6 max-w-[1536px] mx-auto px-4 text-sm" {
                @for (label, href) in LEGAL_LINKS {
                    a   class="hover:text-white transition-colors"
                        href=(href)
                        target="_blank"
                        rel="noopener noreferrer"
                    { (label) }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_every_legal_link() {
        let out = footer().into_string();
        for (label, href) in LEGAL_LINKS {
            assert!(out.contains(href), "missing {href} in {out}");
            assert!(out.contains(label), "missing {label} in {out}");
        }
    }

    #[test]
    fn external_links_do_not_leak_the_opener() {
        // Every footer link leaves the app, so each must carry
        // rel="noopener noreferrer" — the opened page must not reach
        // window.opener of an authenticated session.
        let out = footer().into_string();
        assert_eq!(out.matches(r#"rel="noopener noreferrer""#).count(), LEGAL_LINKS.len(), "{out}");
        assert_eq!(out.matches(r#"target="_blank""#).count(), LEGAL_LINKS.len(), "{out}");
    }

    #[test]
    fn links_inherit_the_footer_colour_rather_than_the_light_surface_link_tile() {
        // Regression guard for a measured WCAG failure: the Mosaic `link` tile is
        // `text-primary-600`, which is 2.35:1 on `bg-slate-800` — below AA. The
        // links must not carry the tile's class, and the footer must supply the
        // colour they inherit.
        let out = footer().into_string();
        assert!(
            !out.contains(r#"class="link"#),
            "footer must not use the light-surface link tile: {out}"
        );
        assert!(out.contains("text-gray-300"), "{out}");
        assert!(out.contains("hover:text-white"), "{out}");
    }
}
