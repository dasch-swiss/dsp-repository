use components::{hero, ComponentBuilder};
use maud::{html, Markup};

use crate::layout::page_layout;

/// News page
pub async fn news() -> Markup {
    let content = html! {
        (hero::hero("Latest News")
            .with_description("Stay updated with the latest developments from DaSCH.")
            .with_id("news-heading")
            .build())
    };

    page_layout("News - DaSCH Swiss", content)
}
