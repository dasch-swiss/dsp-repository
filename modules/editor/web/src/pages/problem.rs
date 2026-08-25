//! The page for a failure that is nobody's fault and nothing the reader can
//! correct.
//!
//! Separate from [`crate::pages::forbidden`] because the two say opposite
//! things. A 403 tells the reader they may not have this; a 500 tells them the
//! editor could not answer. Rendering one through the other means a database
//! that cannot be reached reports itself as a permissions problem — which sends
//! whoever reads it looking for an account that is configured correctly.

use maud::{html, Markup};

/// The body of a 5xx: something broke behind the page.
///
/// No "try again" link, deliberately. Every link this page could offer goes to a
/// page that reads the same database, so a control that leads straight back to
/// the same failure is worse than none. The shell's header is still there.
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
        // The reason this page exists: routed through the 403 page, an
        // unreachable database told an RDU member they did not have access,
        // which is a different problem with a different remedy.
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
