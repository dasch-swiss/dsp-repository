// TODO: find a solution for button-style anchor tags
use maud::{html, Markup};

const BASE_CLASSES: &str =
    "rounded-md px-3 py-2 text-sm font-semibold shadow-xs cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed";

#[derive(Debug, Clone)]
pub enum ButtonVariant {
    Primary,
    Secondary,
}
impl ButtonVariant {
    fn variant_classes(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "bg-indigo-600 text-white hover:bg-indigo-300 focus-visible:outline-2 dark:bg-indigo-500 dark:text-white dark:shadow-none dark:hover:bg-indigo-400 dark:focus-visible:outline-indigo-500",
            ButtonVariant::Secondary => "bg-indigo-300 text-indigo-900 hover:bg-indigo-600 focus-visible:outline-2",
        }
    }

    fn test_id(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "button-primary",
            ButtonVariant::Secondary => "button-secondary",
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
        button type="button" class=(format!("{} {}", BASE_CLASSES, variant.variant_classes())) disabled[disabled] data-testid=(test_id) {
            (text)
        }
    }
}
