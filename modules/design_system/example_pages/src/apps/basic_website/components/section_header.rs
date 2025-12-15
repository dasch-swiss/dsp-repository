use maud::{html, Markup};

/// Section header with optional description
pub fn section_header(heading: impl Into<String>, description: Option<String>) -> Markup {
    let heading_text = heading.into();

    html! {
        div class="mx-auto max-w-2xl text-center" {
            h2 class="text-3xl font-bold tracking-tight text-gray-900 sm:text-4xl dark:text-white" {
                (heading_text)
            }
            @if let Some(desc) = description {
                p class="mt-4 text-lg text-gray-600 dark:text-gray-400" {
                    (desc)
                }
            }
        }
    }
}

/// Section header with description
#[allow(dead_code)]
pub fn section_header_with_description(heading: impl Into<String>, description: impl Into<String>) -> Markup {
    section_header(heading, Some(description.into()))
}

/// Simple section header without description
pub fn section_header_simple(heading: impl Into<String>) -> Markup {
    section_header(heading, None)
}
