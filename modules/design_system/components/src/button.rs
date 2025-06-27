// TODO: verify styling against carbon (including hover state, etc.)
// TODO: find a solution for button-style anchor tags
// TODO: add accessibility features (ARIA states, focus management)
use maud::{html, Markup};

#[derive(Debug, Clone)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Outline,
}

impl ButtonVariant {
    fn css_class(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "dsp-button--primary",
            ButtonVariant::Secondary => "dsp-button--secondary",
            ButtonVariant::Outline => "dsp-button--outline",
        }
    }

    fn test_id(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "button-primary",
            ButtonVariant::Secondary => "button-secondary",
            ButtonVariant::Outline => "button-outline",
        }
    }
}

pub fn button(text: impl Into<String>) -> Markup {
    button_with_variant(text, ButtonVariant::Primary, false)
}

pub fn button_with_variant(text: impl Into<String>, variant: ButtonVariant, disabled: bool) -> Markup {
    button_with_variant_and_testid(text, variant, disabled, None)
}

pub fn button_with_variant_and_testid(
    text: impl Into<String>,
    variant: ButtonVariant,
    disabled: bool,
    custom_test_id: Option<&str>,
) -> Markup {
    let text = text.into();
    let test_id = custom_test_id.unwrap_or(variant.test_id());

    html! {
        button .dsp-button .(variant.css_class()) disabled[disabled] data-testid=(test_id) {
            (text)
        }
    }
}
