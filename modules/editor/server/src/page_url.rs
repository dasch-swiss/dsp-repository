//! The editor's page-URL normalizer, passed into
//! `platform_telemetry::collector::collect_route`.
//!
//! Bounds the `page.url` metric attribute to the editor's own routes so
//! browser metrics stay breakable down by page without exploding cardinality
//! on arbitrary paths. Lives here rather than in `platform-telemetry` because a
//! shared crate cannot know one service's route table (`docs/src/repo_structure.md`
//! → Shared Crates).
//!
//! The real pages today are `/` and the two login screens (see `router.rs`) —
//! the editor is root-mounted, unlike DPE's `/dpe/…` prefix, since it runs on
//! its own hostname. `/projects`, `/review` and
//! `/projects/{shortcode}/sections/{section}` are DEV-6913's job to add here
//! once those pages actually render the beacon script.
//!
//! REVIEW: new full-page routes in `router.rs` need a matching entry here —
//! see `REVIEW.md`.
const KNOWN_ROUTES: &[&str] = &["/", "/login", "/login/code"];

/// Normalize a page URL to a known editor route pattern.
/// Returns "other" for unrecognized paths to prevent metric cardinality explosion.
pub fn normalize_page_url(url: &str) -> &'static str {
    for &route in KNOWN_ROUTES {
        if route == url {
            return route;
        }
    }
    "other"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_routes() {
        assert_eq!(normalize_page_url("/"), "/");
        assert_eq!(normalize_page_url("/login"), "/login");
        assert_eq!(normalize_page_url("/login/code"), "/login/code");
    }

    #[test]
    fn unrelated_paths_return_other() {
        assert_eq!(normalize_page_url("/projects"), "other");
        assert_eq!(normalize_page_url("/healthz"), "other");
        assert_eq!(normalize_page_url("/admin/secret"), "other");
        // Near misses stay bounded: the match is on the whole path, so a query
        // string or a trailing segment does not mint a new attribute value.
        assert_eq!(normalize_page_url("/login?next=/projects"), "other");
        assert_eq!(normalize_page_url("/login/code/extra"), "other");
    }
}
