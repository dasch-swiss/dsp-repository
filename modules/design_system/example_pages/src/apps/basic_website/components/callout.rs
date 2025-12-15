use maud::{html, Markup};

/// Callout box color variants
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum CalloutVariant {
    Blue,
    Green,
    Yellow,
    Red,
    Gray,
}

impl CalloutVariant {
    fn css_classes(&self) -> &'static str {
        match self {
            CalloutVariant::Blue => "bg-blue-50 dark:bg-blue-950",
            CalloutVariant::Green => "bg-green-50 dark:bg-green-950",
            CalloutVariant::Yellow => "bg-yellow-50 dark:bg-yellow-950",
            CalloutVariant::Red => "bg-red-50 dark:bg-red-950",
            CalloutVariant::Gray => "bg-gray-50 dark:bg-gray-800",
        }
    }

    fn text_classes(&self) -> &'static str {
        match self {
            CalloutVariant::Blue => "text-gray-700 dark:text-gray-300",
            CalloutVariant::Green => "text-gray-700 dark:text-gray-300",
            CalloutVariant::Yellow => "text-gray-700 dark:text-gray-300",
            CalloutVariant::Red => "text-gray-700 dark:text-gray-300",
            CalloutVariant::Gray => "text-gray-700 dark:text-gray-300",
        }
    }
}

/// Callout box for additional information
pub fn callout(content: Markup, variant: CalloutVariant) -> Markup {
    let bg_classes = variant.css_classes();
    let text_classes = variant.text_classes();

    html! {
        div class=(format!("rounded-lg p-6 {} {}", bg_classes, text_classes)) {
            (content)
        }
    }
}

/// Callout box with heading and content
#[allow(dead_code)]
pub fn callout_with_heading(heading: impl Into<String>, content: Markup, variant: CalloutVariant) -> Markup {
    let heading_text = heading.into();
    let bg_classes = variant.css_classes();

    html! {
        div class=(format!("rounded-lg p-8 {}", bg_classes)) {
            h2 class="text-xl font-bold text-gray-900 dark:text-white" {
                (heading_text)
            }
            div class="mt-4 text-gray-700 dark:text-gray-300" {
                (content)
            }
        }
    }
}

/// Blue callout (default)
#[allow(dead_code)]
pub fn callout_blue(content: Markup) -> Markup {
    callout(content, CalloutVariant::Blue)
}

/// Gray callout
#[allow(dead_code)]
pub fn callout_gray(content: Markup) -> Markup {
    callout(content, CalloutVariant::Gray)
}
