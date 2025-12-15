use maud::{html, Markup};

/// Info card with heading and description
pub fn info_card(heading: impl Into<String>, description: impl Into<String>) -> Markup {
    let heading_text = heading.into();
    let description_text = description.into();

    html! {
        div class="rounded-lg border border-gray-200 p-6 dark:border-gray-700" {
            h3 class="text-lg font-semibold text-gray-900 dark:text-white" {
                (heading_text)
            }
            p class="mt-2 text-sm text-gray-600 dark:text-gray-400" {
                (description_text)
            }
        }
    }
}
