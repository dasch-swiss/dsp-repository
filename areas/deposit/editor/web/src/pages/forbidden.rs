//! The 403 page.
//!
//! A bare 403 is a dead end in a browser: the reader is signed in and has no
//! control to press, so the status is carried by a page inside the shell with
//! one way back. `/projects` is that way back for every case, since everything
//! rendering this page is already authenticated and `/projects` is reachable by
//! any signed-in account. The message is the caller's: whether a reader may be
//! told what a page holds differs by case.

use maud::{html, Markup};

/// The body of a 403, above a link back to the project list.
pub fn forbidden(message: &str) -> Markup {
    html! {
        div class="max-w-2xl py-12" {
            h1 class="font-display text-2xl mb-2" { "You do not have access to this page" }
            p class="text-gray-600 mb-6" { (message) }
            p {
                a href="/projects" class="underline" { "Go to your projects" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_page_carries_the_callers_message() {
        let out = forbidden("This project is not assigned to you.").into_string();
        assert!(out.contains("This project is not assigned to you."), "{out}");
    }

    #[test]
    fn test_the_page_offers_a_route_back() {
        let out = forbidden("nope").into_string();
        assert!(out.contains(r#"<a href="/projects""#), "{out}");
    }

    #[test]
    fn test_the_message_is_escaped() {
        let out = forbidden("<script>alert(1)</script>").into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }
}
