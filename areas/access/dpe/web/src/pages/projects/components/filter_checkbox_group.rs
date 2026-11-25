use maud::{html, Markup};
use mosaic_tiles::icon::{icon, Info};

/// One checkbox of a facet group: the visible label, the submitted value and whether the
/// current query has it set.
pub struct FilterOption {
    pub label: String,
    pub value: String,
    pub checked: bool,
}

/// A titled group of facet checkboxes: a `fieldset` named by its `legend`, one
/// `label for` + `input type="checkbox" name value` row per option. The rows are form
/// fields, not links, so a crawler finds no URL per option. `name` is the query
/// parameter the form submits.
pub fn filter_checkbox_group(
    title: &str,
    name: &str,
    options: &[FilterOption],
    info_href: Option<&str>,
    info_tooltip: Option<&str>,
) -> Markup {
    html! {
        fieldset class="relative" {
            legend class="dpe-subtitle mb-2" { (title) }
            @if let (Some(href), Some(tooltip)) = (info_href, info_tooltip) {
                div class="group absolute right-0 top-0" {
                    a   href=(href)
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-gray-400 hover:text-primary transition-colors"
                        aria-label="More information"
                    { (icon(Info, "w-4 h-4")) }
                    div class="invisible group-hover:visible absolute right-0 top-full mt-1 w-64 p-2 bg-gray-900 text-white text-xs rounded-lg shadow-lg z-10 pointer-events-none"
                    { (tooltip) }
                }
            }
            div class="space-y-1" {
                @for (index, option) in options.iter().enumerate() {
                    @let id = format!("filter-{name}-{index}");
                    label for=(id) class="flex items-center gap-2 min-h-6 cursor-pointer" {
                        input
                            id=(id)
                            type="checkbox"
                            name=(name)
                            value=(option.value)
                            checked[option.checked]
                            class="w-4 h-4 shrink-0";
                        span { (option.label) }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Vec<FilterOption> {
        vec![
            FilterOption {
                label: "Ongoing".to_string(),
                value: "true".to_string(),
                checked: true,
            },
            FilterOption {
                label: "Finished".to_string(),
                value: "true".to_string(),
                checked: false,
            },
        ]
    }

    #[test]
    fn renders_a_fieldset_named_by_its_legend() {
        let out = filter_checkbox_group("Project Status", "ongoing", &options(), None, None).into_string();
        assert!(out.contains("<fieldset"), "{out}");
        assert!(
            out.contains(r#"<legend class="dpe-subtitle mb-2">Project Status</legend>"#),
            "{out}"
        );
    }

    #[test]
    fn renders_one_labelled_checkbox_per_option_and_no_link() {
        let out = filter_checkbox_group("Project Status", "ongoing", &options(), None, None).into_string();
        assert!(out.contains(r#"<label for="filter-ongoing-0""#), "{out}");
        assert!(
            out.contains(r#"id="filter-ongoing-0" type="checkbox" name="ongoing" value="true" checked"#),
            "{out}"
        );
        assert!(
            out.contains(r#"id="filter-ongoing-1" type="checkbox" name="ongoing" value="true" class"#),
            "{out}"
        );
        assert!(out.contains(">Finished</span>"), "{out}");
        assert!(!out.contains("<a"), "{out}");
    }

    #[test]
    fn keeps_the_focus_indicator() {
        let out = filter_checkbox_group("Project Status", "ongoing", &options(), None, None).into_string();
        assert!(!out.contains("outline-none"), "{out}");
    }

    #[test]
    fn renders_info_tooltip_when_provided() {
        let out = filter_checkbox_group(
            "Access Rights",
            "access_rights",
            &options(),
            Some("https://dasch.swiss/info"),
            Some("Learn more here."),
        )
        .into_string();
        assert!(out.contains(r#"href="https://dasch.swiss/info""#), "{out}");
        assert!(out.contains(r#"target="_blank""#), "{out}");
        assert!(out.contains("Learn more here."), "{out}");
        assert!(out.contains(r#"aria-label="More information""#), "{out}");
    }

    #[test]
    fn omits_info_when_absent() {
        let out = filter_checkbox_group("Type of Data", "type_of_data", &options(), None, None).into_string();
        assert!(!out.contains("More information"), "{out}");
    }
}
