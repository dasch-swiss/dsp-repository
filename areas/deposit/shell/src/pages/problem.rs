//! The page for a failure that is nobody's fault and nothing the reader can
//! correct. Separate from [`crate::pages::forbidden`] because the two say
//! opposite things: a database that cannot be reached must not report itself as
//! a permissions problem.

use maud::{html, Markup};

/// The body of a 5xx. No "try again" link: every link this page could offer
/// reads the same database, so it would lead straight back to the failure.
pub fn unavailable(message: &str) -> Markup {
    html! {
        div class="max-w-2xl py-12" {
            h1 class="font-display text-2xl mb-2" { "This page could not be loaded" }
            p class="text-gray-600" { (message) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_page_does_not_report_a_failure_as_a_permissions_problem() {
        let out = unavailable("The editor could not reach its database.").into_string();
        assert!(!out.contains("access"), "{out}");
        assert!(out.contains("could not be loaded"), "{out}");
        assert!(out.contains("The editor could not reach its database."), "{out}");
    }

    #[test]
    fn test_the_message_is_escaped() {
        let out = unavailable("<script>alert(1)</script>").into_string();
        assert!(!out.contains("<script>alert(1)</script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
    }
}
