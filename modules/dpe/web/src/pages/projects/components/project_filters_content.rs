use maud::{html, Markup};

use super::filter_checkbox_group::filter_checkbox_group;

/// The filter panel body: a "Filters" header (with a "Clear all" link when any
/// filter is active) followed by the access-rights, status, type-of-data and
/// data-language checkbox groups. Shared by the desktop sidebar and the mobile
/// dialog; `dialog_open` adjusts the header layout.
pub fn project_filters_content(
    status_items: &[(String, bool, String)],
    type_of_data_items: &[(String, bool, String)],
    data_language_items: &[(String, bool, String)],
    access_rights_items: &[(String, bool, String)],
    dialog_open: bool,
) -> Markup {
    let any_filter_active = status_items.iter().any(|(_, c, _)| *c)
        || type_of_data_items.iter().any(|(_, c, _)| *c)
        || data_language_items.iter().any(|(_, c, _)| *c)
        || access_rights_items.iter().any(|(_, c, _)| *c);

    html! {
        div class=(if dialog_open { "flex flex-col justify-between mb-4" } else { "flex items-center justify-between" }) {
            h4 class="dpe-title" { "Filters" }
            @if any_filter_active {
                a href="/dpe/projects" class="text-xs text-primary hover:underline" { "Clear all" }
            }
        }
        div class="space-y-4" {
            div {
                (filter_checkbox_group(
                    "Access Rights",
                    access_rights_items,
                    Some("https://dasch.swiss/knowledge-hub/fundamentals-access-rights"),
                    Some("Access rights define how openly the data can be accessed. Learn more here."),
                ))
            }
            div class="border-t border-neutral-200" {}
            div { (filter_checkbox_group("Project Status", status_items, None, None)) }
            div class="border-t border-neutral-200" {}
            div { (filter_checkbox_group("Type of Data", type_of_data_items, None, None)) }
            div class="border-t border-neutral-200" {}
            div { (filter_checkbox_group("Data Language", data_language_items, None, None)) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unchecked(label: &str) -> Vec<(String, bool, String)> {
        vec![(label.to_string(), false, "/dpe/projects".to_string())]
    }

    #[test]
    fn renders_all_four_groups() {
        let out = project_filters_content(&unchecked("a"), &unchecked("b"), &unchecked("c"), &unchecked("d"), false)
            .into_string();
        assert!(out.contains("Access Rights"), "{out}");
        assert!(out.contains("Project Status"), "{out}");
        assert!(out.contains("Type of Data"), "{out}");
        assert!(out.contains("Data Language"), "{out}");
    }

    #[test]
    fn shows_clear_all_only_when_a_filter_is_active() {
        let inactive =
            project_filters_content(&unchecked("a"), &unchecked("b"), &unchecked("c"), &unchecked("d"), false)
                .into_string();
        assert!(!inactive.contains("Clear all"), "{inactive}");

        let active_status = vec![("Ongoing".to_string(), true, "/dpe/projects?ongoing=true".to_string())];
        let active =
            project_filters_content(&active_status, &unchecked("b"), &unchecked("c"), &unchecked("d"), false)
                .into_string();
        assert!(active.contains("Clear all"), "{active}");
    }
}
