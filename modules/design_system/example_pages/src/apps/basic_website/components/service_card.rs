use maud::{html, Markup};

pub struct ServiceCard {
    pub title: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
}

/// Placeholder service card component
pub fn service_card(card: &ServiceCard) -> Markup {
    html! {
        div class="rounded-lg bg-white p-8 shadow-sm ring-1 ring-gray-900/5 dark:bg-gray-800 dark:ring-white/10" {
            div class="mb-4 text-4xl" {
                (card.icon)
            }
            h3 class="text-lg font-semibold text-gray-900 dark:text-white" {
                (card.title)
            }
            p class="mt-4 text-sm text-gray-600 dark:text-gray-400" {
                (card.description)
            }
        }
    }
}
