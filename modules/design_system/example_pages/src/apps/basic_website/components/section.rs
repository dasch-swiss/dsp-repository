use maud::{html, Markup};

/// Standard content section with white background
pub fn content_section(content: Markup) -> Markup {
    html! {
        section class="bg-white py-24 sm:py-32 dark:bg-gray-900" {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                (content)
            }
        }
    }
}

/// Content section with aria-labelledby for accessibility
pub fn content_section_labeled(label_id: &str, content: Markup) -> Markup {
    html! {
        section class="bg-white py-24 sm:py-32 dark:bg-gray-900" aria-labelledby=(label_id) {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                (content)
            }
        }
    }
}

/// Content section with gray background
#[allow(dead_code)]
pub fn content_section_gray(content: Markup) -> Markup {
    html! {
        section class="bg-gray-50 py-24 sm:py-32 dark:bg-gray-800" {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                (content)
            }
        }
    }
}

/// Content section with reduced padding (for projects page)
#[allow(dead_code)]
pub fn content_section_compact(content: Markup) -> Markup {
    html! {
        section class="bg-white py-12 sm:py-16 dark:bg-gray-900" {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                (content)
            }
        }
    }
}

/// Content section with gray background and label
pub fn content_section_gray_labeled(label_id: &str, content: Markup) -> Markup {
    html! {
        section class="bg-gray-50 py-24 sm:py-32 dark:bg-gray-800" aria-labelledby=(label_id) {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                (content)
            }
        }
    }
}

/// Content section with compact padding and label
pub fn content_section_compact_labeled(label_id: &str, content: Markup) -> Markup {
    html! {
        section class="bg-white py-12 sm:py-16 dark:bg-gray-900" aria-labelledby=(label_id) {
            div class="mx-auto max-w-7xl px-6 lg:px-8" {
                (content)
            }
        }
    }
}
