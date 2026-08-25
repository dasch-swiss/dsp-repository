//! The 403 page.
//!
//! REQ-1.3 specifies a status, not a page. A bare 403 is a dead end in a
//! browser: the reader is signed in, has done nothing wrong, and has no control
//! to press. So the status is carried by a page inside the shell, with one
//! obvious way back.
//!
//! `/projects` is that way back for every case, and can be, because everything
//! that renders this page is already authenticated — an unauthenticated request
//! is redirected to login long before it reaches here — and `/projects` is
//! reachable by any signed-in account, RDU included.
//!
//! The message is the caller's. What a reader may be told differs by case: that
//! a project is not theirs is safe to say, because they had to name the
//! shortcode to get here, whereas an RDU-only page should not describe what it
//! holds.

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
        // The whole reason REQ-1.3's status is rendered as a page: a bare 403
        // leaves a signed-in reader with nothing to press.
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
