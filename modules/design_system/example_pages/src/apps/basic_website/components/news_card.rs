use maud::{html, Markup};

pub struct NewsCard {
    pub title: &'static str,
    pub date: &'static str,
    pub description: &'static str,
}

/// Placeholder news card component
pub fn news_card(news: &NewsCard) -> Markup {
    html! {
        article class="flex flex-col items-start justify-between" {
            div class="flex items-center gap-x-4 text-xs" {
                time datetime=(news.date) class="text-gray-500 dark:text-gray-400" {
                    (news.date)
                }
            }
            div class="group relative" {
                h3 class="mt-3 text-lg font-semibold leading-6 text-gray-900 dark:text-white" {
                    (news.title)
                }
                p class="mt-5 line-clamp-3 text-sm leading-6 text-gray-600 dark:text-gray-400" {
                    (news.description)
                }
            }
        }
    }
}
