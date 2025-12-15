use maud::{html, Markup};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum InfoBoxVariant {
    Default,   // gray border
    Highlight, // indigo border with background
}

#[allow(dead_code)]
impl InfoBoxVariant {
    fn css_class(&self) -> &'static str {
        match self {
            InfoBoxVariant::Default => "rounded-lg border border-gray-200 p-6 dark:border-gray-700",
            InfoBoxVariant::Highlight => {
                "rounded-lg border border-indigo-200 bg-indigo-50 p-6 dark:border-indigo-800 dark:bg-indigo-950"
            }
        }
    }
}

/// Bordered info box with title and content
#[allow(dead_code)]
pub fn info_box(title: impl Into<String>, content: Markup, variant: InfoBoxVariant) -> Markup {
    let title = title.into();

    html! {
        div class=(variant.css_class()) {
            h3 class="text-xl font-semibold text-gray-900 dark:text-white" { (title) }
            (content)
        }
    }
}

/// Info box without title (just content)
#[allow(dead_code)]
pub fn info_box_simple(content: Markup, variant: InfoBoxVariant) -> Markup {
    html! {
        div class=(variant.css_class()) {
            (content)
        }
    }
}
