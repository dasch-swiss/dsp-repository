use maud::{html, Markup};
use mosaic_tiles::icon::{icon, Tune};

use super::project_filters_content::project_filters_content;

/// The mobile "Filters" trigger plus, when `dialog_open`, a slide-in panel
/// (with backdrop) holding the filter content. Open/close is driven by the
/// `dialog` query param, so it works without JavaScript.
#[allow(clippy::too_many_arguments)]
pub fn mobile_filters_button(
    status_items: &[(String, bool, String)],
    type_of_data_items: &[(String, bool, String)],
    data_language_items: &[(String, bool, String)],
    access_rights_items: &[(String, bool, String)],
    dialog_open: bool,
    open_dialog_href: &str,
    close_dialog_href: &str,
) -> Markup {
    html! {
        a href=(open_dialog_href) class="btn btn-outline flex items-center gap-2 cursor-pointer" {
            (icon(Tune, "w-5 h-5"))
            span class="text-sm font-medium" { "Filters" }
        }

        @if dialog_open {
            // Backdrop. A redundant click-to-close target: the dedicated "Close
            // filters" button below carries the accessible name, so this is
            // hidden from assistive tech and removed from the tab order.
            a   href=(close_dialog_href)
                aria-hidden="true"
                tabindex="-1"
                class="fixed inset-0 bg-black/40 z-40 lg:hidden" {}
            // Panel
            div class="fixed right-0 top-0 bottom-0 w-full md:w-96 bg-white z-50 overflow-y-auto lg:hidden"
            {
                div class="relative p-4" {
                    a   href=(close_dialog_href)
                        aria-label="Close filters"
                        class="btn btn-ghost size-8 justify-center rounded-full p-0 absolute right-2 top-2 cursor-pointer"
                    { "✕" }
                    ({
                        project_filters_content(
                            status_items,
                            type_of_data_items,
                            data_language_items,
                            access_rights_items,
                            true,
                        )
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> Vec<(String, bool, String)> {
        vec![]
    }

    #[test]
    fn closed_renders_only_the_trigger() {
        let out = mobile_filters_button(
            &empty(),
            &empty(),
            &empty(),
            &empty(),
            false,
            "/dpe/projects?dialog=true",
            "/dpe/projects",
        )
        .into_string();
        assert!(out.contains(r#"href="/dpe/projects?dialog=true""#), "{out}");
        assert!(out.contains("Filters"), "{out}");
        // No slide-in panel when closed.
        assert!(!out.contains("fixed inset-0"), "{out}");
    }

    #[test]
    fn open_renders_backdrop_and_panel() {
        let out = mobile_filters_button(
            &empty(),
            &empty(),
            &empty(),
            &empty(),
            true,
            "/dpe/projects?dialog=true",
            "/dpe/projects",
        )
        .into_string();
        assert!(out.contains("fixed inset-0 bg-black/40"), "backdrop missing: {out}");
        assert!(out.contains("✕"), "close button missing: {out}");
    }
}
