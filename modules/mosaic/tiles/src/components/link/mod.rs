//! Link tile: an anchor that can optionally be styled as a button.
//!
//! `link(label, href)` returns a [`LinkBuilder`]; set options with chained
//! methods and either splice it into `html!` directly (it implements [`Render`])
//! or call `.build()` for a standalone `Markup`. See
//! `docs/src/mosaic/component-api-conventions.md`.

use maud::{html, Markup, Render};

use crate::builder::ComponentBuilder;
use crate::components::button::ButtonVariant;

/// Builder for an `<a>` element. Construct with [`link`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct LinkBuilder {
    label: Markup,
    href: String,
    as_button: Option<ButtonVariant>,
    target: Option<String>,
    rel: Option<String>,
    aria_label: Option<String>,
    aria_current: Option<String>,
    disabled: bool,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start a link with the given label and destination. `label` is `impl Render`
/// (a string or richer markup); `href` is any string-like value.
pub fn link(label: impl Render, href: impl Into<String>) -> LinkBuilder {
    LinkBuilder {
        label: label.render(),
        href: href.into(),
        as_button: None,
        target: None,
        rel: None,
        aria_label: None,
        aria_current: None,
        disabled: false,
        id: None,
        test_id: None,
    }
}

impl LinkBuilder {
    /// Render the anchor styled as a button with the given variant.
    pub fn as_button(mut self, variant: ButtonVariant) -> Self {
        self.as_button = Some(variant);
        self
    }

    /// Set the `target` attribute (e.g. `_blank`).
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Set the `rel` attribute (e.g. `noopener noreferrer`).
    pub fn rel(mut self, rel: impl Into<String>) -> Self {
        self.rel = Some(rel.into());
        self
    }

    /// Open in a new tab safely: sets `target="_blank"` and
    /// `rel="noopener noreferrer"` (the `noopener` guards against the opened
    /// page reaching `window.opener`). Prefer this over setting `target`
    /// manually for external links.
    pub fn external(mut self) -> Self {
        self.target = Some("_blank".to_string());
        self.rel = Some("noopener noreferrer".to_string());
        self
    }

    /// Set an `aria-label` (for icon-only or image-only links).
    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// Set `aria-current` (e.g. `"page"` for the active link in a paginated or
    /// navigational list), so assistive technology can announce which item
    /// represents the current location.
    pub fn aria_current(mut self, aria_current: impl Into<String>) -> Self {
        self.aria_current = Some(aria_current.into());
        self
    }

    /// Mark the link disabled: drops the `href`, sets `aria-disabled`, and (in
    /// button mode) a `-1` tabindex.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    fn markup(&self) -> Markup {
        let href = (!self.disabled).then_some(self.href.as_str());
        let aria_disabled = self.disabled.then_some("true");
        let tabindex = (self.as_button.is_some() && self.disabled).then_some("-1");
        let class = match self.as_button {
            Some(variant) => variant.css_class().to_string(),
            None => format!("link {}", if self.disabled { "link-disabled" } else { "" }),
        };
        html! {
            a   href=[href]
                class=(class)
                target=[self.target.as_deref()]
                rel=[self.rel.as_deref()]
                aria-label=[self.aria_label.as_deref()]
                aria-current=[self.aria_current.as_deref()]
                aria-disabled=[aria_disabled]
                tabindex=[tabindex]
                id=[self.id.as_deref()]
                data-testid=[self.test_id.as_deref()]
            { (self.label) }
        }
    }
}

impl ComponentBuilder for LinkBuilder {
    fn id_mut(&mut self) -> &mut Option<String> {
        &mut self.id
    }

    fn test_id_mut(&mut self) -> &mut Option<String> {
        &mut self.test_id
    }

    fn build(self) -> Markup {
        self.markup()
    }
}

impl Render for LinkBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_renders_anchor_with_link_class() {
        let out = link("Go", "/x").build().into_string();
        assert!(out.starts_with("<a "), "{out}");
        assert!(out.contains(r#"href="/x""#));
        assert!(out.contains(r#"class="link "#), "{out}");
        assert!(out.contains(">Go</a>"));
    }

    #[test]
    fn label_accepts_markup() {
        let out = link(
            html! {
                span { "x" }
            },
            "/x",
        )
        .build()
        .into_string();
        assert!(out.contains("<span>x</span>"), "{out}");
    }

    #[test]
    fn disabled_link_omits_href_and_marks_aria() {
        let out = link("", "/x").disabled().build().into_string();
        assert!(!out.contains("href="), "disabled link must drop href: {out}");
        assert!(out.contains("link-disabled"), "{out}");
        assert!(out.contains(r#"aria-disabled="true""#), "{out}");
        assert!(!out.contains("tabindex="), "link mode has no tabindex: {out}");
    }

    #[test]
    fn as_button_uses_variant_class() {
        let out = link("Act", "/x").as_button(ButtonVariant::Primary).build().into_string();
        assert!(out.contains(r#"class="btn btn-primary""#), "{out}");
        assert!(!out.contains("link-disabled"));
    }

    #[test]
    fn disabled_button_link_sets_negative_tabindex() {
        let out = link("", "/x").as_button(ButtonVariant::Ghost).disabled().build().into_string();
        assert!(out.contains(r#"tabindex="-1""#), "{out}");
        assert!(!out.contains("href="), "{out}");
    }

    #[test]
    fn passes_through_target_rel_aria_label() {
        let out = link("", "/x")
            .target("_blank")
            .rel("noopener noreferrer")
            .aria_label("External")
            .build()
            .into_string();
        assert!(out.contains(r#"target="_blank""#));
        assert!(out.contains(r#"rel="noopener noreferrer""#));
        assert!(out.contains(r#"aria-label="External""#));
    }

    #[test]
    fn aria_current_is_emitted() {
        let out = link("2", "/x").aria_current("page").build().into_string();
        assert!(out.contains(r#"aria-current="page""#), "{out}");
    }

    #[test]
    fn external_sets_blank_target_and_noopener_rel() {
        let out = link("Docs", "/x").external().build().into_string();
        assert!(out.contains(r#"target="_blank""#), "{out}");
        assert!(out.contains(r#"rel="noopener noreferrer""#), "{out}");
    }

    #[test]
    fn id_and_test_id_are_emitted() {
        let out = link("Go", "/x").with_id("l").with_test_id("link-x").build().into_string();
        assert!(out.contains(r#"id="l""#), "{out}");
        assert!(out.contains(r#"data-testid="link-x""#), "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = link("Go", "/x").as_button(ButtonVariant::Primary).build().into_string();
        let spliced = html! {
            (link("Go", "/x").as_button(ButtonVariant::Primary))
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}
