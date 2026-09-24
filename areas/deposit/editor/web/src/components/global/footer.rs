use maud::{html, Markup};

/// Legal links every DaSCH service carries; the legal minimum for an
/// authenticated tool.
const LEGAL_LINKS: [(&str, &str); 3] = [
    ("Legal Notice", "https://dasch.swiss/legal-notice"),
    ("Privacy Policy", "https://dasch.swiss/privacy-policy"),
    ("Impressum", "https://dasch.swiss/impressum"),
];

/// The global footer: a slim row of legal links.
///
/// Plain anchors inheriting `text-gray-300` rather than the Mosaic `link` tile:
/// the tile's `text-primary-600` measures 2.35:1 against `bg-slate-800`, below
/// WCAG 2.1 AA's 4.5:1; the inherited grey is 9.93:1.
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
        // Every link leaves the app; the opened page must not reach window.opener.
        let out = footer().into_string();
        assert_eq!(out.matches(r#"rel="noopener noreferrer""#).count(), LEGAL_LINKS.len(), "{out}");
        assert_eq!(out.matches(r#"target="_blank""#).count(), LEGAL_LINKS.len(), "{out}");
    }

    #[test]
    fn links_inherit_the_footer_colour_rather_than_the_light_surface_link_tile() {
        let out = footer().into_string();
        assert!(
            !out.contains(r#"class="link"#),
            "footer must not use the light-surface link tile: {out}"
        );
        assert!(out.contains("text-gray-300"), "{out}");
        assert!(out.contains("hover:text-white"), "{out}");
    }
}
