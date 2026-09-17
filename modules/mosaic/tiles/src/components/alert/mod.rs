//! Alert tile: a bordered, tinted block carrying a message, with an optional
//! bold title above it.
//!
//! See `docs/src/mosaic/component-api-conventions.md`.
//!
//! ## Why the variant decides the ARIA role
//!
//! `role="alert"` marks an assertive live region. Announcement is unreliable for
//! a message already in the parsed document, so the role is not a promise that a
//! rejected form is spoken the instant it arrives; it is what identifies the
//! block as an error to a reader who reaches it, and what makes the message
//! announce itself once these banners are updated by a fragment rather than a
//! full re-render. That fits `Danger`, which reports a submission that did not
//! go through, and is wrong for a block merely stating a consequence — so
//! `Warning`, `Info` and `Success` carry no role.

use maud::{html, Markup, Render};

use crate::builder::ComponentBuilder;

#[derive(Clone, Copy, Debug, Default)]
pub enum AlertVariant {
    #[default]
    Info,
    Success,
    Warning,
    Danger,
}

impl AlertVariant {
    /// Complete, literal class string, so Tailwind's source scan sees it.
    pub fn css_class(self) -> &'static str {
        match self {
            AlertVariant::Info => "alert-info",
            AlertVariant::Success => "alert-success",
            AlertVariant::Warning => "alert-warning",
            AlertVariant::Danger => "alert-danger",
        }
    }

    /// The ARIA role this variant renders with, if any. See the module docs.
    pub fn aria_role(self) -> Option<&'static str> {
        match self {
            AlertVariant::Danger => Some("alert"),
            AlertVariant::Info | AlertVariant::Success | AlertVariant::Warning => None,
        }
    }
}

/// Builder for a `<div class="alert …">`. Construct with [`alert`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct AlertBuilder {
    content: Markup,
    title: Option<Markup>,
    variant: AlertVariant,
    extra_classes: String,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start an alert wrapping the given content. `content` is `impl Render`, so it
/// accepts a plain string, markup, or another builder.
pub fn alert(content: impl Render) -> AlertBuilder {
    AlertBuilder {
        content: content.render(),
        title: None,
        variant: AlertVariant::default(),
        extra_classes: String::new(),
        id: None,
        test_id: None,
    }
}

impl AlertBuilder {
    /// Set the colour variant (default `Info`), which also decides the ARIA role.
    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Add a bold title line above the content.
    pub fn title(mut self, title: impl Render) -> Self {
        self.title = Some(title.render());
        self
    }

    /// Append extra utility classes after the variant classes.
    ///
    /// Not the place for bottom spacing — the tile carries `mb-4` itself. A
    /// caller needing a different margin uses Tailwind's `!` modifier (`!mb-6`):
    /// two same-property utilities are resolved by CSS source order rather than
    /// by their order in the attribute, so a plain `mb-6` may or may not win.
    pub fn class(mut self, classes: impl Into<String>) -> Self {
        self.extra_classes = classes.into();
        self
    }

    fn markup(&self) -> Markup {
        let class = if self.extra_classes.is_empty() {
            format!("alert {}", self.variant.css_class())
        } else {
            format!("alert {} {}", self.variant.css_class(), self.extra_classes)
        };
        html! {
            div class=(class)
                role=[self.variant.aria_role()]
                id=[self.id.as_deref()]
                data-testid=[self.test_id.as_deref()]
            {
                @if let Some(title) = &self.title {
                    p class="alert-title" { (title) }
                }
                (self.content)
            }
        }
    }
}

impl ComponentBuilder for AlertBuilder {
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

impl Render for AlertBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_class_mapping_is_complete_and_literal() {
        assert_eq!(AlertVariant::Info.css_class(), "alert-info");
        assert_eq!(AlertVariant::Success.css_class(), "alert-success");
        assert_eq!(AlertVariant::Warning.css_class(), "alert-warning");
        assert_eq!(AlertVariant::Danger.css_class(), "alert-danger");
    }

    #[test]
    fn danger_is_the_only_variant_that_interrupts() {
        assert_eq!(AlertVariant::Danger.aria_role(), Some("alert"));
        assert_eq!(AlertVariant::Info.aria_role(), None);
        assert_eq!(AlertVariant::Success.aria_role(), None);
        assert_eq!(AlertVariant::Warning.aria_role(), None);
    }

    #[test]
    fn danger_alert_renders_the_role_and_the_message() {
        let out = alert("That address is already in use.")
            .variant(AlertVariant::Danger)
            .build()
            .into_string();
        assert!(out.contains(r#"role="alert""#), "{out}");
        assert!(out.contains(r#"class="alert alert-danger""#), "{out}");
        assert!(out.contains("That address is already in use."), "{out}");
    }

    #[test]
    fn a_non_danger_alert_carries_no_role_attribute() {
        let out = alert("x").variant(AlertVariant::Warning).build().into_string();
        assert!(!out.contains("role="), "{out}");
    }

    #[test]
    fn default_alert_is_info_and_titleless() {
        let out = alert("x").build().into_string();
        assert!(out.contains(r#"class="alert alert-info""#), "{out}");
        assert!(!out.contains("alert-title"), "no title element when unset: {out}");
    }

    #[test]
    fn title_renders_above_the_content() {
        let out = alert("The work is kept.")
            .variant(AlertVariant::Warning)
            .title("What this leaves behind")
            .build()
            .into_string();
        let title_at = out.find("What this leaves behind").expect("title missing");
        let body_at = out.find("The work is kept.").expect("body missing");
        assert!(title_at < body_at, "title must precede the content: {out}");
        assert!(out.contains(r#"<p class="alert-title">"#), "{out}");
    }

    #[test]
    fn content_and_title_accept_markup() {
        let body = html! {
            ul {
                li { "one" }
            }
        };
        let out = alert(body)
            .title(html! {
                span { "heads up" }
            })
            .build()
            .into_string();
        assert!(out.contains("<ul><li>one</li></ul>"), "{out}");
        assert!(out.contains("<span>heads up</span>"), "{out}");
    }

    #[test]
    fn extra_classes_follow_the_variant() {
        let out = alert("x").variant(AlertVariant::Danger).class("mt-8").build().into_string();
        assert!(out.contains(r#"class="alert alert-danger mt-8""#), "{out}");
    }

    #[test]
    fn the_tile_carries_its_own_bottom_margin() {
        // Asserted against the stylesheet because that is the only place the CSS
        // and the markup meet: `.alert` is what carries the margin.
        let out = alert("x").build().into_string();
        assert!(out.contains(r#"class="alert alert-info""#), "{out}");
        let css = include_str!("alert.css");
        assert!(
            css.contains("@apply mb-4 rounded border"),
            "the `.alert` rule owns the margin: {css}"
        );
    }

    #[test]
    fn id_and_test_id_are_emitted() {
        let out = alert("x").with_id("a").with_test_id("alert-x").build().into_string();
        assert!(out.contains(r#"id="a""#), "{out}");
        assert!(out.contains(r#"data-testid="alert-x""#), "{out}");
    }

    #[test]
    fn omits_optional_attributes_when_unset() {
        let out = alert("x").build().into_string();
        assert!(!out.contains("id="), "{out}");
        assert!(!out.contains("data-testid="), "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = alert("x").variant(AlertVariant::Success).build().into_string();
        let spliced = html! {
            (alert("x").variant(AlertVariant::Success))
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}
