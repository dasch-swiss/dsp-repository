//! Breadcrumb navigation tile.

use maud::{html, Markup};

/// Render the breadcrumb `<nav>` wrapping a `<ol>` of items.
pub fn breadcrumb(items: Markup) -> Markup {
    html! {
        nav aria-label="Breadcrumb" class="breadcrumb" {
            ol class="breadcrumb-list" { (items) }
        }
    }
}

/// Render a single breadcrumb item.
///
/// With `href`, renders a link; without, renders the current-page text.
pub fn breadcrumb_item(href: Option<&str>, content: Markup) -> Markup {
    html! {
        li class="breadcrumb-item" {
            @match href {
                Some(h) => {
                    a href=(h) class="breadcrumb-link" { (content) }
                }
                None => {
                    span class="breadcrumb-current" aria-current="page" { (content) }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breadcrumb_wraps_nav_and_list() {
        let out = breadcrumb(html! {
            "items"
        })
        .into_string();
        assert!(out.contains(r#"<nav aria-label="Breadcrumb" class="breadcrumb">"#), "{out}");
        assert!(out.contains(r#"<ol class="breadcrumb-list">items</ol>"#), "{out}");
    }

    #[test]
    fn item_with_href_renders_link() {
        let out = breadcrumb_item(
            Some("/home"),
            html! {
                "Home"
            },
        )
        .into_string();
        assert!(out.contains(r#"<a href="/home" class="breadcrumb-link">Home</a>"#), "{out}");
    }

    #[test]
    fn item_without_href_is_current_page() {
        let out = breadcrumb_item(
            None,
            html! {
                "Here"
            },
        )
        .into_string();
        assert!(
            out.contains(r#"<span class="breadcrumb-current" aria-current="page">Here</span>"#),
            "{out}"
        );
    }
}
