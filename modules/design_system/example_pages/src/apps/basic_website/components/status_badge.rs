use maud::{html, Markup};

/// Status badge color variants
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum StatusBadgeVariant {
    Green,
    Blue,
    Yellow,
    Red,
    Gray,
}

impl StatusBadgeVariant {
    fn css_classes(&self) -> &'static str {
        match self {
            StatusBadgeVariant::Green => {
                "bg-green-50 text-green-700 ring-green-600/20 dark:bg-green-900 dark:text-green-300"
            }
            StatusBadgeVariant::Blue => "bg-blue-50 text-blue-700 ring-blue-600/20 dark:bg-blue-900 dark:text-blue-300",
            StatusBadgeVariant::Yellow => {
                "bg-yellow-50 text-yellow-700 ring-yellow-600/20 dark:bg-yellow-900 dark:text-yellow-300"
            }
            StatusBadgeVariant::Red => "bg-red-50 text-red-700 ring-red-600/20 dark:bg-red-900 dark:text-red-300",
            StatusBadgeVariant::Gray => "bg-gray-50 text-gray-700 ring-gray-600/20 dark:bg-gray-900 dark:text-gray-300",
        }
    }
}

/// Status badge component
pub fn status_badge(text: impl Into<String>, variant: StatusBadgeVariant) -> Markup {
    let text_str = text.into();
    let classes = variant.css_classes();

    html! {
        span class=(format!("inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ring-1 ring-inset {}", classes)) {
            (text_str)
        }
    }
}

/// Active status badge (green)
pub fn status_badge_active() -> Markup {
    status_badge("Active", StatusBadgeVariant::Green)
}

/// Inactive status badge (gray)
#[allow(dead_code)]
pub fn status_badge_inactive() -> Markup {
    status_badge("Inactive", StatusBadgeVariant::Gray)
}
