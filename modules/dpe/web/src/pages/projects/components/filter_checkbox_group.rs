use maud::{html, Markup};
use mosaic_tiles::icon::{icon, Info};

/// A titled group of filter checkboxes. Each item is `(label, checked, href)`;
/// the whole row is a link that navigates to `href` (a server-side filter
/// toggle). The checkbox is `pointer-events-none` so the link handles the click.
pub fn filter_checkbox_group(
    title: &str,
    items: &[(String, bool, String)],
    info_href: Option<&str>,
    info_tooltip: Option<&str>,
) -> Markup {
    html! {
        div {
            div class="flex items-center justify-between mb-2" {
                span class="dpe-subtitle" { (title) }
                @if let (Some(href), Some(tooltip)) = (info_href, info_tooltip) {
                    div class="group relative" {
                        a href=(href) target="_blank" rel="noopener noreferrer"
                          class="text-gray-400 hover:text-primary transition-colors"
                          aria-label="More information" {
                            (icon(Info, "w-4 h-4"))
                        }
                        div class="invisible group-hover:visible absolute right-0 top-full mt-1 w-64 p-2 bg-gray-900 text-white text-xs rounded-lg shadow-lg z-10 pointer-events-none" {
                            (tooltip)
                        }
                    }
                }
            }
            div class="space-y-2" {
                @for (label, checked, href) in items {
                    a href=(href) class="flex items-center gap-2 cursor-pointer"
                      aria-current=[checked.then_some("true")] {
                        input type="checkbox" class="w-4 h-4 pointer-events-none"
                              checked[*checked] aria-label=(label);
                        span { (label) }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<(String, bool, String)> {
        vec![
            ("Ongoing".to_string(), true, "/dpe/projects?ongoing=true".to_string()),
            ("Finished".to_string(), false, "/dpe/projects?finished=true".to_string()),
        ]
    }

    #[test]
    fn renders_title_and_item_links() {
        let out = filter_checkbox_group("Project Status", &items(), None, None).into_string();
        assert!(out.contains("Project Status"), "{out}");
        assert!(out.contains(r#"href="/dpe/projects?ongoing=true""#), "{out}");
        assert!(out.contains(">Ongoing</span>"), "{out}");
    }

    #[test]
    fn marks_checked_items() {
        let out = filter_checkbox_group("Project Status", &items(), None, None).into_string();
        // The "Ongoing" row is checked, "Finished" is not.
        assert!(out.contains(r#"aria-current="true""#), "{out}");
        assert!(out.contains("checked"), "{out}");
    }

    #[test]
    fn renders_info_tooltip_when_provided() {
        let out = filter_checkbox_group(
            "Access Rights",
            &items(),
            Some("https://dasch.swiss/info"),
            Some("Learn more here."),
        )
        .into_string();
        assert!(out.contains(r#"href="https://dasch.swiss/info""#), "{out}");
        assert!(out.contains("Learn more here."), "{out}");
        assert!(out.contains(r#"aria-label="More information""#), "{out}");
    }

    #[test]
    fn omits_info_when_absent() {
        let out = filter_checkbox_group("Type of Data", &items(), None, None).into_string();
        assert!(!out.contains("More information"), "{out}");
    }
}
