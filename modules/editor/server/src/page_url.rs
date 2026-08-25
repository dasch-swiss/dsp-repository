//! The editor's page-URL normalizer, passed into
//! `platform_telemetry::collector::collect_route`.
//!
//! Bounds the `page.url` metric attribute to the editor's own routes so
//! browser metrics stay breakable down by page without exploding cardinality
//! on arbitrary paths. Lives here rather than in `platform-telemetry` because a
//! shared crate cannot know one service's route table (`docs/src/repo_structure.md`
//! → Shared Crates).
//!
//! The editor is root-mounted, unlike DPE's `/dpe/…` prefix, since it runs on
//! its own hostname. `/depositors`, `/review`, `/review/{shortcode}` and
//! `/projects/{shortcode}/sections/{section}` join this list as those surfaces
//! land.
//!
//! `/` is deliberately absent. It is a redirect, so it never renders the beacon
//! script and no beacon can report it — the same reason `/` and `/dpe` are
//! absent from DPE's list. The normalizer still answers sanely if one somehow
//! did.
//!
//! The beacon reports `location.pathname`, so a value never carries a query.
//! `/login?next=…` therefore does not reach here, and the login pages stay
//! attributed to `/login` rather than falling into `other` — pinned by a test
//! below, because the day the beacon starts sending a full URL is the day every
//! login page view silently becomes `other`.
//!
//! REVIEW: new full-page routes in `router.rs` need a matching entry here —
//! see `REVIEW.md`.
const KNOWN_ROUTES: &[&str] = &["/login", "/login/code", "/projects"];

/// Normalize a page URL to a known editor route pattern.
/// Returns "other" for unrecognized paths to prevent metric cardinality explosion.
pub fn normalize_page_url(url: &str) -> &'static str {
    for &route in KNOWN_ROUTES {
        if route == url {
            return route;
        }
    }
    // Pattern match for `/projects/{shortcode}`, without allocating. A shortcode
    // is an unbounded set, so letting one through verbatim is the cardinality
    // explosion this module exists to prevent.
    if let Some(rest) = url.strip_prefix("/projects/") {
        if !rest.is_empty() && !rest.contains('/') {
            return "/projects/{shortcode}";
        }
    }
    "other"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_routes() {
        assert_eq!(normalize_page_url("/login"), "/login");
        assert_eq!(normalize_page_url("/login/code"), "/login/code");
        assert_eq!(normalize_page_url("/projects"), "/projects");
    }

    #[test]
    fn the_shortcode_segment_collapses_to_its_pattern() {
        // A shortcode is an unbounded set. Letting one through verbatim is
        // exactly the cardinality explosion this module exists to prevent.
        assert_eq!(normalize_page_url("/projects/0801"), "/projects/{shortcode}");
        assert_eq!(normalize_page_url("/projects/0801a"), "/projects/{shortcode}");
    }

    #[test]
    fn the_root_is_a_redirect_and_so_not_a_page() {
        // It renders no beacon script, so no beacon can report it — same reason
        // `/` and `/dpe` are absent from DPE's list.
        assert_eq!(normalize_page_url("/"), "other");
    }

    #[test]
    fn unrelated_paths_return_other() {
        assert_eq!(normalize_page_url("/healthz"), "other");
        assert_eq!(normalize_page_url("/admin/secret"), "other");
        // Near misses stay bounded: the match is on the whole path, so a query
        // string or a trailing segment does not mint a new attribute value.
        assert_eq!(normalize_page_url("/login?next=/projects"), "other");
        assert_eq!(normalize_page_url("/login/code/extra"), "other");
        assert_eq!(normalize_page_url("/projects/0801/sections/general"), "other");
        assert_eq!(normalize_page_url("/projects/"), "other");
    }

    #[test]
    fn a_beacon_never_carries_a_query_so_login_keeps_its_own_attribution() {
        // `telemetry.js` sends `location.pathname`. If that ever becomes a full
        // URL, `/login?next=…` starts arriving here and every login page view
        // silently becomes `other` — this is the test that would fail first.
        assert_eq!(normalize_page_url("/login"), "/login");
        assert_ne!(normalize_page_url("/login?next=/projects"), "/login");
    }
}
