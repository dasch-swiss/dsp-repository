use maud::{html, Markup};

/// Article data structure
#[derive(Debug, Clone)]
pub struct Article {
    pub title: String,
    pub href: String,
}

/// Article category section with articles list
pub fn article_category(
    heading: impl Into<String>,
    article_count: usize,
    articles: &[Article],
    is_last: bool,
) -> Markup {
    let heading_text = heading.into();
    let border_class = if is_last {
        "pb-8"
    } else {
        "border-b border-gray-200 pb-8 dark:border-gray-700"
    };

    html! {
        div class=(border_class) {
            h2 class="text-2xl font-bold text-gray-900 dark:text-white" {
                (heading_text)
            }
            p class="mt-2 text-sm text-gray-500 dark:text-gray-400" {
                (article_count) " " @if article_count == 1 { "article" } @else { "articles" }
            }
            ul class="mt-4 space-y-2" {
                @for article in articles {
                    li {
                        a href=(article.href) class="text-indigo-600 hover:text-indigo-500 dark:text-indigo-400" {
                            (article.title)
                        }
                    }
                }
            }
        }
    }
}
