//! Card tile: a container with a variant plus header/body/footer partials.
//!
//! `card(content)` returns a [`CardBuilder`]; set options with chained methods
//! and either splice it into `html!` directly (it implements [`Render`]) or call
//! `.build()` for a standalone `Markup`. See
//! `docs/src/mosaic/component-api-conventions.md`.

use maud::{html, Markup, Render};

use crate::builder::ComponentBuilder;

#[derive(Clone, Copy, Debug, Default)]
pub enum CardVariant {
    #[default]
    Default,
    Bordered,
    Elevated,
    AutoHover,
}

impl CardVariant {
    pub fn css_class(self) -> &'static str {
        match self {
            CardVariant::Default => "card-default",
            CardVariant::Bordered => "card-bordered",
            CardVariant::Elevated => "card-elevated",
            CardVariant::AutoHover => "card-autohover",
        }
    }
}

/// Builder for the outer card container. Construct with [`card`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct CardBuilder {
    content: Markup,
    variant: CardVariant,
    extra_classes: String,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start a card wrapping the given content. `content` is `impl Render`, so it
/// accepts markup, another builder, or a string.
pub fn card(content: impl Render) -> CardBuilder {
    CardBuilder {
        content: content.render(),
        variant: CardVariant::default(),
        extra_classes: String::new(),
        id: None,
        test_id: None,
    }
}

impl CardBuilder {
    /// Set the visual variant (default `Default`).
    pub fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Append extra utility classes after the variant classes.
    pub fn class(mut self, classes: impl Into<String>) -> Self {
        self.extra_classes = classes.into();
        self
    }

    fn markup(&self) -> Markup {
        let class = if self.extra_classes.is_empty() {
            format!("card {}", self.variant.css_class())
        } else {
            format!("card {} {}", self.variant.css_class(), self.extra_classes)
        };
        html! {
            div class=(class) id=[self.id.as_deref()] data-testid=[self.test_id.as_deref()] {
                (self.content)
            }
        }
    }
}

impl ComponentBuilder for CardBuilder {
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

impl Render for CardBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

/// Render the card header partial.
#[must_use]
pub fn card_header(content: impl Render) -> Markup {
    html! {
        div class="card-header" { (content) }
    }
}

/// Render the card body partial.
#[must_use]
pub fn card_body(content: impl Render) -> Markup {
    html! {
        div class="card-body" { (content) }
    }
}

/// Render the card body partial with extra classes on the body element itself
/// (e.g. `flex-1` so the body grows within a full-height card).
#[must_use]
pub fn card_body_with_class(class: &str, content: impl Render) -> Markup {
    html! {
        div class=(format!("card-body {class}")) { (content) }
    }
}

/// Render the card footer partial.
#[must_use]
pub fn card_footer(content: impl Render) -> Markup {
    html! {
        div class="card-footer" { (content) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_class_mapping() {
        assert_eq!(CardVariant::Default.css_class(), "card-default");
        assert_eq!(CardVariant::Bordered.css_class(), "card-bordered");
        assert_eq!(CardVariant::Elevated.css_class(), "card-elevated");
        assert_eq!(CardVariant::AutoHover.css_class(), "card-autohover");
    }

    #[test]
    fn default_card_wraps_content() {
        let out = card("body").build().into_string();
        assert!(out.contains(r#"class="card card-default""#), "{out}");
        assert!(out.contains(">body</div>"));
    }

    #[test]
    fn variant_and_extra_class_compose() {
        let out = card("x").variant(CardVariant::Bordered).class("mt-4").build().into_string();
        assert!(out.contains(r#"class="card card-bordered mt-4""#), "{out}");
    }

    #[test]
    fn content_accepts_markup() {
        let out = card(html! {
            span { "hi" }
        })
        .build()
        .into_string();
        assert!(out.contains("<span>hi</span>"), "{out}");
    }

    #[test]
    fn id_and_test_id_are_emitted() {
        let out = card("x").with_id("c").with_test_id("card-x").build().into_string();
        assert!(out.contains(r#"id="c""#), "{out}");
        assert!(out.contains(r#"data-testid="card-x""#), "{out}");
    }

    #[test]
    fn omits_optional_attributes_when_unset() {
        let out = card("x").build().into_string();
        assert!(!out.contains("id="), "{out}");
        assert!(!out.contains("data-testid="), "{out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        let built = card("x").variant(CardVariant::Elevated).build().into_string();
        let spliced = html! {
            (card("x").variant(CardVariant::Elevated))
        }
        .into_string();
        assert_eq!(built, spliced);
    }

    #[test]
    fn body_header_footer_partials() {
        assert!(card_header(html! { "h" })
            .into_string()
            .contains(r#"class="card-header">h</div>"#));
        assert!(card_body(html! { "b" }).into_string().contains(r#"class="card-body">b</div>"#));
        assert!(card_footer(html! { "f" })
            .into_string()
            .contains(r#"class="card-footer">f</div>"#));
    }

    #[test]
    fn body_with_class_appends_to_card_body() {
        let out = card_body_with_class(
            "flex-1 flex flex-col",
            html! {
                "b"
            },
        )
        .into_string();
        assert!(out.contains(r#"class="card-body flex-1 flex flex-col">b</div>"#), "{out}");
    }
}
