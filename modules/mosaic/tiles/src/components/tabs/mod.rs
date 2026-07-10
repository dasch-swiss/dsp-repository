//! Tabs tile with CSS-only switching via hidden radio inputs.
//!
//! `tabs` renders the container; each `tab` renders a sibling triple
//! (radio input, label, panel). All tabs in a group must share the same
//! `name` so the radios form a mutually exclusive set. Switching is pure CSS
//! (see `tabs.css`), no JavaScript.
//!
//! `tab(name, value, label, panel)` returns a [`TabBuilder`]; set the optional
//! icon/checked state with chained methods and splice it into `html!` directly
//! (it implements [`Render`]). Being a compound of three sibling elements, the
//! tab has no single `id`/`data-testid` target, so it does not implement
//! `ComponentBuilder`; it provides an inherent `.build()` for standalone use.
//!
//! NOTE (DEV-6642): this CSS-only radio tile emits no ARIA tab-widget semantics
//! (`role="tablist"`/`"tab"`/`"tabpanel"`, `aria-selected`, `aria-controls`), so
//! screen readers announce it as a radio group, not a tab interface. The DPE
//! hand-rolls a second, richer tab implementation in
//! `dpe-web`'s `pages/project/components/project_details_tabs` — a Datastar/SSE,
//! URL-addressable tablist that *does* carry full ARIA + keyboard nav. These two
//! should probably converge: grow this tile into the canonical, ARIA-complete
//! tab component (with a `.selected()` semantic method) and have the DPE consume
//! it, rather than maintaining two. Deferred — it's a design decision, not a
//! mechanical change (CSS-radio vs Datastar-morph switching differ).
//!
//! ```
//! use mosaic_tiles::tabs::{tab, tabs};
//! use maud::html;
//!
//! let markup = tabs(html! {
//!     (tab("g", "a", "A", html! { "Panel A" }).checked())
//!     (tab("g", "b", "B", html! { "Panel B" }))
//! });
//! ```

use maud::{html, Markup, Render};

use crate::components::icon::{icon, IconData};

/// Render the tabs container wrapping the given `tab` triples.
#[must_use]
pub fn tabs(content: impl Render) -> Markup {
    html! {
        div class="tabs" { (content) }
    }
}

/// Builder for a single tab (radio input + label + panel). Construct with [`tab`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct TabBuilder {
    name: String,
    value: String,
    label: Markup,
    panel: Markup,
    icon: Option<IconData>,
    checked: bool,
}

/// Start a tab in group `name` with the given `value`, `label`, and panel
/// content. All tabs in a group must share the same `name`. `label` is
/// `impl Render` (a string or richer markup), consistent with the other tiles.
pub fn tab(name: impl Into<String>, value: impl Into<String>, label: impl Render, panel: impl Render) -> TabBuilder {
    TabBuilder {
        name: name.into(),
        value: value.into(),
        label: label.render(),
        panel: panel.render(),
        icon: None,
        checked: false,
    }
}

impl TabBuilder {
    /// Show an icon before the label.
    pub fn icon(mut self, icon: IconData) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Mark this tab initially selected.
    pub fn checked(mut self) -> Self {
        self.checked = true;
        self
    }

    fn markup(&self) -> Markup {
        let input_id = format!("{}-{}", self.name, self.value);
        html! {
            input
                type="radio"
                class="tab-input"
                id=(input_id)
                name=(self.name)
                value=(self.value)
                checked[self.checked];
            label class="tab-label" for=(input_id) {
                @if let Some(tab_icon) = self.icon { (icon(tab_icon, "tab-icon")) }
                span { (self.label) }
            }
            div class="tab-panel" { (self.panel) }
        }
    }

    /// Render to `Markup` (for standalone use; inside `html!`, splice directly).
    pub fn build(self) -> Markup {
        self.markup()
    }
}

impl Render for TabBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::icon::IconSearch;

    #[test]
    fn container_wraps_content() {
        let out = tabs(html! {
            "x"
        })
        .into_string();
        assert!(out.contains(r#"<div class="tabs">x</div>"#), "{out}");
    }

    #[test]
    fn tab_renders_input_label_panel_triple() {
        let out = tab(
            "grp",
            "one",
            "One",
            html! {
                "Panel"
            },
        )
        .build()
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
        let out = tab("g", "v", "L", html! {}).checked().build().into_string();
        assert!(out.contains("checked"), "{out}");
    }

    #[test]
    fn unchecked_tab_omits_checked_attribute() {
        let out = tab("g", "v", "L", html! {}).build().into_string();
        assert!(!out.contains("checked"), "{out}");
    }

    #[test]
    fn tab_with_icon_renders_tab_icon_svg() {
        let out = tab("g", "v", "L", html! {}).icon(IconSearch).build().into_string();
        // The tab icon is rendered through the shared `icon()` tile, so it
        // carries the base `icon` class and is `aria-hidden`.
        assert!(out.contains(r#"<svg class="icon tab-icon""#), "{out}");
        assert!(out.contains(r#"aria-hidden="true""#), "{out}");
    }

    #[test]
    fn tab_label_accepts_markup() {
        let out = tab(
            "g",
            "v",
            html! {
                span { "Rich" }
            },
            html! {},
        )
        .build()
        .into_string();
        assert!(out.contains("<span>Rich</span>"), "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = tab(
            "g",
            "v",
            "L",
            html! {
                "p"
            },
        )
        .checked()
        .build()
        .into_string();
        let spliced = html! {
            ({
                tab(
                        "g",
                        "v",
                        "L",
                        html! {
                            "p"
                        },
                    )
                    .checked()
            })
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}
