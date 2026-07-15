//! Breadcrumb navigation tile.

use maud::{html, Markup, Render};

/// Render the breadcrumb `<nav>` wrapping a `<ol>` of items.
#[must_use]
pub fn breadcrumb(items: impl Render) -> Markup {
    html! {
        nav aria-label="Breadcrumb" class="breadcrumb" {
            ol class="breadcrumb-list" { (items) }
        }
    }
}

/// A breadcrumb item that links to `href`.
#[must_use]
pub fn breadcrumb_item(href: impl Into<String>, label: impl Render) -> Markup {
    html! {
        li class="breadcrumb-item" {
            a href=(href.into()) class="breadcrumb-link" { (label) }
        }
    }
}

/// The final, current-page breadcrumb item: unlinked text carrying
/// `aria-current="page"`.
#[must_use]
pub fn breadcrumb_current(label: impl Render) -> Markup {
    html! {
        li class="breadcrumb-item" {
            span class="breadcrumb-current" aria-current="page" { (label) }
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
    fn item_renders_link() {
        let out = breadcrumb_item(
            "/home",
            html! {
                "Home"
            },
        )
        .into_string();
        assert!(out.contains(r#"<a href="/home" class="breadcrumb-link">Home</a>"#), "{out}");
    }

    #[test]
    fn current_item_is_current_page() {
        let out = breadcrumb_current(html! {
            "Here"
        })
        .into_string();
        assert!(
            out.contains(r#"<span class="breadcrumb-current" aria-current="page">Here</span>"#),
            "{out}"
        );
    }
}
