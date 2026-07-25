//! Badge tile with variant and size enums mapped to complete, literal classes.
//!
//! `badge(label)` returns a [`BadgeBuilder`]; set options with chained methods
//! and either splice it into `html!` directly (it implements [`Render`]) or call
//! `.build()` for a standalone `Markup`. See
//! `docs/src/mosaic/component-api-conventions.md`.

use maud::{html, Markup, Render};

use crate::builder::ComponentBuilder;

#[derive(Clone, Copy, Debug, Default)]
pub enum BadgeVariant {
    #[default]
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Info,
}

impl BadgeVariant {
    pub fn css_class(self) -> &'static str {
        match self {
            BadgeVariant::Primary => "badge-primary",
            BadgeVariant::Secondary => "badge-secondary",
            BadgeVariant::Success => "badge-success",
            BadgeVariant::Warning => "badge-warning",
            BadgeVariant::Danger => "badge-danger",
            BadgeVariant::Info => "badge-info",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum BadgeSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl BadgeSize {
    pub fn css_class(self) -> &'static str {
        match self {
            BadgeSize::Small => "badge-sm",
            BadgeSize::Medium => "badge-md",
            BadgeSize::Large => "badge-lg",
        }
    }
}

/// Builder for a `<span class="badge …">`. Construct with [`badge`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct BadgeBuilder {
    label: Markup,
    variant: BadgeVariant,
    size: BadgeSize,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start a badge with the given label. The label is `impl Render`, so it accepts
/// a plain string or richer markup (e.g. an icon plus text).
pub fn badge(label: impl Render) -> BadgeBuilder {
    BadgeBuilder {
        label: label.render(),
        variant: BadgeVariant::default(),
        size: BadgeSize::default(),
        id: None,
        test_id: None,
    }
}

impl BadgeBuilder {
    /// Set the colour variant (default `Primary`).
    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the size (default `Medium`).
    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }

    fn markup(&self) -> Markup {
        html! {
            span
                class=({
                    format!(
                        "badge {} {}",
                        self.variant.css_class(),
                        self.size.css_class(),
                    )
                })
                id=[self.id.as_deref()]
                data-testid=[self.test_id.as_deref()]
            { (self.label) }
        }
    }
}

impl ComponentBuilder for BadgeBuilder {
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

impl Render for BadgeBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_class_mapping() {
        assert_eq!(BadgeVariant::Primary.css_class(), "badge-primary");
        assert_eq!(BadgeVariant::Secondary.css_class(), "badge-secondary");
        assert_eq!(BadgeVariant::Success.css_class(), "badge-success");
        assert_eq!(BadgeVariant::Warning.css_class(), "badge-warning");
        assert_eq!(BadgeVariant::Danger.css_class(), "badge-danger");
        assert_eq!(BadgeVariant::Info.css_class(), "badge-info");
    }

    #[test]
    fn size_class_mapping() {
        assert_eq!(BadgeSize::Small.css_class(), "badge-sm");
        assert_eq!(BadgeSize::Medium.css_class(), "badge-md");
        assert_eq!(BadgeSize::Large.css_class(), "badge-lg");
    }

    #[test]
    fn default_badge_is_primary_medium() {
        let out = badge("New").build().into_string();
        assert!(out.contains(r#"class="badge badge-primary badge-md""#), "{out}");
        assert!(out.contains(">New</span>"));
    }

    #[test]
    fn label_accepts_markup() {
        let out = badge(html! {
            span { "x" }
        })
        .build()
        .into_string();
        assert!(out.contains("<span>x</span>"), "{out}");
    }

    #[test]
    fn composes_variant_and_size() {
        let out = badge("")
            .variant(BadgeVariant::Danger)
            .size(BadgeSize::Large)
            .build()
            .into_string();
        assert!(out.contains(r#"class="badge badge-danger badge-lg""#), "{out}");
    }

    #[test]
    fn id_and_test_id_are_emitted() {
        let out = badge("x").with_id("b").with_test_id("badge-x").build().into_string();
        assert!(out.contains(r#"id="b""#), "{out}");
        assert!(out.contains(r#"data-testid="badge-x""#), "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = badge("x").variant(BadgeVariant::Success).build().into_string();
        let spliced = html! {
            (badge("x").variant(BadgeVariant::Success))
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}
