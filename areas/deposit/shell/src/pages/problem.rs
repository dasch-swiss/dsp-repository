//! The page for a failure that is nobody's fault and nothing the reader can
//! correct. Separate from [`crate::pages::forbidden`] because the two say
//! opposite things: a database that cannot be reached must not report itself as
//! a permissions problem.

use maud::Markup;

/// The body of a 500: every caller renders it with `StatusCode::INTERNAL_SERVER_ERROR`, so a
/// caller using another status must change the code shown here.
///
/// No "try again" link: a link back to the failing page leads straight to the failure again.
pub fn unavailable(message: &str) -> Markup {
    super::status_page("500", "This page could not be loaded", message)
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

    #[test]
    fn test_the_status_code_precedes_the_heading_and_the_page_links_to_projects() {
        let out = unavailable("down").into_string();
        crate::pages::assert_status_page(&out, "500");
    }

    #[test]
    fn test_the_page_has_exactly_one_link() {
        let out = unavailable("down").into_string();
        assert_eq!(out.matches("<a ").count(), 1, "{out}");
    }
}
