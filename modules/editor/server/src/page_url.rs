//! The editor's page-URL normalizer, passed into
//! `platform_telemetry::collector::collect_route`.
//!
//! Bounds the `page.url` metric attribute to the editor's own routes so
//! browser metrics stay breakable down by page without exploding cardinality
//! on arbitrary paths. Lives here rather than in `platform-telemetry` because a
//! shared crate cannot know one service's route table (`docs/src/repo_structure.md`
//! → Shared Crates).
//!
//! Only `/` is a real page today (see `router.rs`) — the editor is
//! root-mounted, unlike DPE's `/dpe/…` prefix, since it runs on its own
//! hostname. `/projects`, `/review` and `/projects/{shortcode}/sections/{section}`
//! are DEV-6913's job to add here once those pages actually render the beacon
//! script.
//!
//! REVIEW: new full-page routes in `router.rs` need a matching entry here —
//! see `REVIEW.md`.
const KNOWN_ROUTES: &[&str] = &["/"];

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
    fn known_route() {
        assert_eq!(normalize_page_url("/"), "/");
    }

    #[test]
    fn unrelated_paths_return_other() {
        assert_eq!(normalize_page_url("/projects"), "other");
        assert_eq!(normalize_page_url("/healthz"), "other");
        assert_eq!(normalize_page_url("/admin/secret"), "other");
    }
}
