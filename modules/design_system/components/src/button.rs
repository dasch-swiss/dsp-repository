// TODO: find a solution for button-style anchor tags
use maud::{html, Markup};

const BASE_CLASSES: &str =
    "rounded-md px-3 py-2 text-sm font-semibold shadow-xs cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed";

const ICON_BUTTON_BASE_CLASSES: &str =
    "rounded-md p-2 text-sm font-semibold cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 dark:hover:bg-gray-800";

const DEFAULT_ICON_BUTTON_COLOR: &str = "text-gray-900 dark:text-gray-300";

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

// TODO: Create type-safe DataStar action wrapper to replace raw string onclick handlers
// This would provide compile-time validation for DataStar actions like:
// - DataStarAction::ConsoleLog(msg)
// - DataStarAction::Get { url, options }
// - DataStarAction::Post { url, options }
// See: https://data-star.dev/ for DataStar action syntax

pub fn button(text: impl Into<String>) -> Markup {
    button_with_variant(text, ButtonVariant::Primary, false, None)
}

pub fn button_with_variant(
    text: impl Into<String>,
    variant: ButtonVariant,
    disabled: bool,
    onclick: Option<&str>,
) -> Markup {
    button_with_variant_and_testid(text, variant, disabled, onclick, None)
}

pub fn button_with_variant_and_testid(
    text: impl Into<String>,
    variant: ButtonVariant,
    disabled: bool,
    onclick: Option<&str>,
    custom_test_id: Option<&str>,
) -> Markup {
    let text = text.into();
    let test_id = custom_test_id.unwrap_or(variant.test_id());

    html! {
        button
            type="button"
            class=(format!("{} {}", BASE_CLASSES, variant.variant_classes()))
            disabled[disabled]
            data-on-click=[onclick]
            data-testid=(test_id)
        {
            (text)
        }
    }
}

/// Creates an icon button with default styling
///
/// Icon buttons are square buttons containing only an icon, commonly used for
/// compact actions like closing dialogs, opening menus, or triggering popovers.
///
/// Uses subtle gray colors with hover states by default. For custom colors,
/// use `icon_button_with_color()`.
///
/// # Example
/// ```rust
/// use components::{button, icon, IconType};
///
/// let close_button = button::icon_button(icon::icon(IconType::Close), false, None);
/// let menu_trigger = button::icon_button(icon::icon(IconType::Hamburger), false, None);
///
/// // With onclick handler
/// let clickable = button::icon_button(
///     icon::icon(IconType::Star),
///     false,
///     Some("console.log('Icon clicked!')")
/// );
/// ```
pub fn icon_button(icon: Markup, disabled: bool, onclick: Option<&str>) -> Markup {
    icon_button_with_color(icon, None, disabled, onclick)
}

/// Creates an icon button with custom color classes
///
/// Allows full control over icon button colors via Tailwind CSS classes.
/// The color classes override the default gray colors.
///
/// # Example
/// ```rust
/// use components::{button, icon, IconType};
///
/// // Icon button with custom colors
/// let custom_button = button::icon_button_with_color(
///     icon::icon(IconType::Star),
///     Some("text-yellow-500 hover:bg-yellow-50 dark:hover:bg-yellow-950"),
///     false,
///     None
/// );
///
/// // Indigo colored icon button with onclick
/// let indigo_button = button::icon_button_with_color(
///     icon::icon(IconType::Close),
///     Some("text-indigo-600 hover:bg-indigo-50 dark:text-indigo-400"),
///     false,
///     Some("console.log('Custom close!')")
/// );
/// ```
pub fn icon_button_with_color(
    icon: Markup,
    color_class: Option<&str>,
    disabled: bool,
    onclick: Option<&str>,
) -> Markup {
    let color = color_class.unwrap_or(DEFAULT_ICON_BUTTON_COLOR);

    html! {
        button
            type="button"
            class=(format!("{} {}", ICON_BUTTON_BASE_CLASSES, color))
            disabled[disabled]
            data-on-click=[onclick]
            data-testid="icon-button"
        {
            (icon)
        }
    }
}
