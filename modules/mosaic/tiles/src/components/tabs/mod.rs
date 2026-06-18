//! Tabs tile with CSS-only switching via hidden radio inputs.
//!
//! `tabs` renders the container; each `tab` renders a sibling triple
//! (radio input, label, panel). All tabs in a group must share the same
//! `name` so the radios form a mutually exclusive set. Switching is pure CSS
//! (see `tabs.css`), no JavaScript.
//!
//! ```
//! use mosaic_tiles::tabs::{tab, tabs, TabProps};
//! use maud::html;
//!
//! let markup = tabs(html! {
//!     (tab(TabProps { name: "g", value: "a", label: "A", checked: true, ..Default::default() },
//!          html! { "Panel A" }))
//!     (tab(TabProps { name: "g", value: "b", label: "B", ..Default::default() },
//!          html! { "Panel B" }))
//! });
//! ```

use maud::{html, Markup, PreEscaped};

use crate::components::icon::IconData;

/// Render the tabs container wrapping the given `tab` triples.
pub fn tabs(content: Markup) -> Markup {
    html! {
        div class="tabs" style="border-width: 0" {
            (content)
        }
    }
}

#[derive(Default)]
pub struct TabProps<'a> {
    /// Radio group name — must be the same for all tabs in a group.
    pub name: &'a str,
    /// Unique identifier for this tab within the group.
    pub value: &'a str,
    /// The tab label text.
    pub label: &'a str,
    /// Optional icon rendered before the label.
    pub icon: Option<IconData>,
    /// Whether this tab is initially selected.
    pub checked: bool,
}

/// Render a single tab: the radio input, its label, and the content panel.
pub fn tab(props: TabProps, content: Markup) -> Markup {
    let input_id = format!("{}-{}", props.name, props.value);
    html! {
        input type="radio" class="tab-input" id=(input_id) name=(props.name) value=(props.value) checked[props.checked];
        label class="tab-label" for=(input_id) {
            @if let Some(icon) = props.icon {
                svg class="tab-icon" xmlns="http://www.w3.org/2000/svg" viewBox=[icon.view_box] fill="currentColor" {
                    (PreEscaped(icon.data))
                }
            }
            span { (props.label) }
        }
        div class="tab-panel" { (content) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::icon::IconSearch;

    #[test]
    fn container_wraps_content() {
        let out = tabs(html! { "x" }).into_string();
        assert!(out.contains(r#"<div class="tabs" style="border-width: 0">x</div>"#), "{out}");
    }

    #[test]
    fn tab_renders_input_label_panel_triple() {
        let out = tab(
            TabProps {
                name: "grp",
                value: "one",
                label: "One",
                ..Default::default()
            },
            html! { "Panel" },
        )
        .into_string();
        assert!(
            out.contains(r#"<input type="radio" class="tab-input" id="grp-one" name="grp" value="one">"#),
            "{out}"
        );
        assert!(out.contains(r#"<label class="tab-label" for="grp-one">"#), "{out}");
        assert!(out.contains("<span>One</span>"), "{out}");
        assert!(out.contains(r#"<div class="tab-panel">Panel</div>"#), "{out}");
    }

    #[test]
    fn checked_tab_has_checked_attribute() {
        let out = tab(
            TabProps {
                name: "g",
                value: "v",
                label: "L",
                checked: true,
                ..Default::default()
            },
            html! {},
        )
        .into_string();
        assert!(out.contains("checked"), "{out}");
    }

    #[test]
    fn unchecked_tab_omits_checked_attribute() {
        let out = tab(TabProps { name: "g", value: "v", label: "L", ..Default::default() }, html! {}).into_string();
        assert!(!out.contains("checked"), "{out}");
    }

    #[test]
    fn tab_with_icon_renders_tab_icon_svg() {
        let out = tab(
            TabProps {
                name: "g",
                value: "v",
                label: "L",
                icon: Some(IconSearch),
                ..Default::default()
            },
            html! {},
        )
        .into_string();
        assert!(out.contains(r#"<svg class="tab-icon""#), "{out}");
    }
}
