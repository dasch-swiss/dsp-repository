//! Button tile and the shared `ButtonVariant` / `ButtonType` enums.
//!
//! `button(label)` returns a [`ButtonBuilder`]; set options with chained methods
//! and either splice it into `html!` directly (it implements [`Render`]) or call
//! `.build()` for a standalone `Markup`. See
//! `docs/src/mosaic/component-api-conventions.md`.
//!
//! `ButtonVariant::css_class` returns a complete, literal class string per arm
//! so Tailwind's source scanner can see every class (never interpolate class
//! fragments). The `link` tile reuses `ButtonVariant` to render anchors styled
//! as buttons.

use maud::{html, Markup, Render};

use crate::builder::ComponentBuilder;

#[derive(Clone, Copy, Debug, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Outline,
    Ghost,
    Soft,
}

impl ButtonVariant {
    /// Complete, literal class string (includes the base `btn`).
    pub fn css_class(self) -> &'static str {
        match self {
            ButtonVariant::Primary => "btn btn-primary",
            ButtonVariant::Secondary => "btn btn-secondary",
            ButtonVariant::Outline => "btn btn-outline",
            ButtonVariant::Ghost => "btn btn-ghost",
            ButtonVariant::Soft => "btn btn-soft",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ButtonType {
    #[default]
    Button,
    Reset,
    Submit,
}

impl ButtonType {
    /// The `type` attribute value.
    pub fn as_str(self) -> &'static str {
        match self {
            ButtonType::Button => "button",
            ButtonType::Reset => "reset",
            ButtonType::Submit => "submit",
        }
    }
}

/// Builder for a `<button>`. Construct with [`button`].
#[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
pub struct ButtonBuilder {
    label: Markup,
    variant: ButtonVariant,
    button_type: ButtonType,
    disabled: bool,
    extra_classes: String,
    id: Option<String>,
    test_id: Option<String>,
}

/// Start a button with the given label. The label is `impl Render`, so it
/// accepts a plain string or richer markup (e.g. an icon plus text).
pub fn button(label: impl Render) -> ButtonBuilder {
    ButtonBuilder {
        label: label.render(),
        variant: ButtonVariant::default(),
        button_type: ButtonType::default(),
        disabled: false,
        extra_classes: String::new(),
        id: None,
        test_id: None,
    }
}

impl ButtonBuilder {
    /// Set the visual variant (default `Primary`).
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the HTML `type` attribute (default `Button`).
    pub fn button_type(mut self, button_type: ButtonType) -> Self {
        self.button_type = button_type;
        self
    }

    /// Mark the button disabled.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Append extra utility classes after the variant classes.
    pub fn class(mut self, classes: impl Into<String>) -> Self {
        self.extra_classes = classes.into();
        self
    }

    fn markup(&self) -> Markup {
        let class = if self.extra_classes.is_empty() {
            self.variant.css_class().to_string()
        } else {
            format!("{} {}", self.variant.css_class(), self.extra_classes)
        };
        html! {
            button
                class=(class)
                type=(self.button_type.as_str())
                id=[self.id.as_deref()]
                data-testid=[self.test_id.as_deref()]
                disabled[self.disabled]
            { (self.label) }
        }
    }
}

impl ComponentBuilder for ButtonBuilder {
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

impl Render for ButtonBuilder {
    fn render(&self) -> Markup {
        self.markup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_class_mapping_is_complete_and_literal() {
        assert_eq!(ButtonVariant::Primary.css_class(), "btn btn-primary");
        assert_eq!(ButtonVariant::Secondary.css_class(), "btn btn-secondary");
        assert_eq!(ButtonVariant::Outline.css_class(), "btn btn-outline");
        assert_eq!(ButtonVariant::Ghost.css_class(), "btn btn-ghost");
        assert_eq!(ButtonVariant::Soft.css_class(), "btn btn-soft");
    }

    #[test]
    fn type_mapping() {
        assert_eq!(ButtonType::Button.as_str(), "button");
        assert_eq!(ButtonType::Reset.as_str(), "reset");
        assert_eq!(ButtonType::Submit.as_str(), "submit");
    }

    #[test]
    fn default_button_is_primary_and_typed_button() {
        let out = button("Click").build().into_string();
        assert!(out.contains(r#"class="btn btn-primary""#), "missing variant class: {out}");
        assert!(out.contains(r#"type="button""#));
        assert!(out.contains(">Click</button>"));
        assert!(!out.contains("disabled"), "default button must not be disabled: {out}");
    }

    #[test]
    fn label_accepts_markup_not_only_strings() {
        // `impl Render` must accept nested markup, not just a string label.
        let out = button(html! {
            span { "hi" }
        })
        .build()
        .into_string();
        assert!(out.contains("<span>hi</span>"), "{out}");
    }

    #[test]
    fn disabled_renders_boolean_attribute() {
        let out = button("x").disabled().build().into_string();
        assert!(out.contains("disabled"), "expected disabled attribute: {out}");
    }

    #[test]
    fn submit_type_and_extra_classes() {
        let out = button("x")
            .button_type(ButtonType::Submit)
            .class("w-full")
            .build()
            .into_string();
        assert!(out.contains(r#"type="submit""#));
        assert!(out.contains(r#"class="btn btn-primary w-full""#), "{out}");
    }

    #[test]
    fn id_and_test_id_are_emitted() {
        let out = button("x").with_id("save").with_test_id("save-btn").build().into_string();
        assert!(out.contains(r#"id="save""#), "{out}");
        assert!(out.contains(r#"data-testid="save-btn""#), "{out}");
    }

    #[test]
    fn omits_optional_attributes_when_unset() {
        let out = button("x").build().into_string();
        assert!(!out.contains("id="), "no id when unset: {out}");
        assert!(!out.contains("data-testid="), "no testid when unset: {out}");
    }

    #[test]
    fn renders_identically_whether_spliced_or_built() {
        // Splicing via `Render` must match `.build()` — the whole point of the
        // Render impl is that `.build()` is unnecessary inside `html!`.
        let built = button("Go").variant(ButtonVariant::Secondary).build().into_string();
        let spliced = html! {
            (button("Go").variant(ButtonVariant::Secondary))
        }
        .into_string();
        assert_eq!(built, spliced);
    }
}
