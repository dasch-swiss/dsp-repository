// TODO: find a solution for button-style anchor tags
use maud::{html, Markup};

#[derive(Debug, Clone)]
pub enum ButtonVariant {
    Primary,
    Secondary,
}

impl ButtonVariant {
    fn css_classes(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-xs hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 dark:bg-indigo-500 dark:shadow-none dark:hover:bg-indigo-400 dark:focus-visible:outline-indigo-500",
            ButtonVariant::Secondary => "rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-xs inset-ring inset-ring-gray-300 hover:bg-gray-50 dark:bg-white/10 dark:text-white dark:shadow-none dark:inset-ring-white/5 dark:hover:bg-white/20",
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
        button type="button" class=(variant.css_classes()) disabled[disabled] data-testid=(test_id) {
            (text)
        }
    }
}
