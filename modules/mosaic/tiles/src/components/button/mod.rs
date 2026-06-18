//! Button tile and the shared `ButtonVariant` / `ButtonType` enums.
//!
//! `ButtonVariant::css_class` returns a complete, literal class string per arm
//! so Tailwind's source scanner can see every class (never interpolate class
//! fragments). The `link` tile reuses `ButtonVariant` to render anchors styled
//! as buttons.

use maud::{html, Markup};

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

#[derive(Default)]
pub struct ButtonProps<'a> {
    pub variant: ButtonVariant,
    pub button_type: ButtonType,
    pub disabled: bool,
    pub extra_classes: &'a str,
}

/// Render a `<button>` with the variant classes and the given `label` content.
pub fn button(props: ButtonProps, label: Markup) -> Markup {
    html! {
        button class=(format!("{} {}", props.variant.css_class(), props.extra_classes))
               type=(props.button_type.as_str())
               disabled[props.disabled] {
            (label)
        }
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
        let out = button(ButtonProps::default(), html! { "Click" }).into_string();
        assert!(out.contains(r#"class="btn btn-primary "#), "missing variant class: {out}");
        assert!(out.contains(r#"type="button""#));
        assert!(out.contains(">Click</button>"));
        assert!(!out.contains("disabled"), "default button must not be disabled: {out}");
    }

    #[test]
    fn disabled_renders_boolean_attribute() {
        let out = button(ButtonProps { disabled: true, ..Default::default() }, html! {}).into_string();
        assert!(out.contains("disabled"), "expected disabled attribute: {out}");
    }

    #[test]
    fn submit_type_and_extra_classes() {
        let out = button(
            ButtonProps {
                button_type: ButtonType::Submit,
                extra_classes: "w-full",
                ..Default::default()
            },
            html! {},
        )
        .into_string();
        assert!(out.contains(r#"type="submit""#));
        assert!(out.contains("w-full"));
    }
}
