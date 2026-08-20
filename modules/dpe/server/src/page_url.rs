//! DPE's page-URL normalizer, passed into `platform_telemetry::collector::collect_route`.
//!
//! Bounds the `page.url` metric attribute to DPE's own routes so browser
//! metrics stay breakable down by page without exploding cardinality on
//! arbitrary paths. Lives here rather than in `platform-telemetry` because a
//! shared crate cannot know one service's route table (`docs/src/repo_structure.md`
//! → Shared Crates).
//!
//! Only DPE's full-page routes belong here — see `router.rs` for the route
//! table. Fragment (`/dpe/projects/{id}/tab/{tab}`, `/dpe/projects/search`),
//! JSON API (`/dpe/api/v2/...`) and OAI-PMH (`/dpe/oai`) routes never render
//! the beacon script, so a beacon can never report their path; `/` and `/dpe`
//! are 308 redirects and likewise never render it.
//!
//! REVIEW: new full-page routes in `router.rs` need a matching entry here —
//! see `REVIEW.md`.
const KNOWN_ROUTES: &[&str] = &["/dpe/projects", "/dpe/about"];

/// Normalize a page URL to a known DPE route pattern.
/// Returns "other" for unrecognized paths to prevent metric cardinality explosion.
pub fn normalize_page_url(url: &str) -> &'static str {
    for &route in KNOWN_ROUTES {
        if route == url {
            return route;
        }
    }
    // Pattern match for /dpe/projects/{id} without allocating.
    match url.strip_prefix("/dpe/projects/") {
        Some(id) if !id.is_empty() && !id.contains('/') => "/dpe/projects/{id}",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_routes() {
        assert_eq!(normalize_page_url("/dpe/projects"), "/dpe/projects");
        assert_eq!(normalize_page_url("/dpe/about"), "/dpe/about");
    }

    #[test]
    fn project_detail() {
        assert_eq!(normalize_page_url("/dpe/projects/0803"), "/dpe/projects/{id}");
    }

    #[test]
    fn project_detail_never_leaks_a_nested_path() {
        assert_eq!(normalize_page_url("/dpe/projects/0803/extra"), "other");
    }

    #[test]
    fn root_and_redirect_targets_are_not_known_routes() {
        // "/" and "/dpe" are 308 redirects — they never render the beacon script,
        // so a real beacon can never report them, but the normalizer must still
        // answer sanely if one somehow did.
        assert_eq!(normalize_page_url("/"), "other");
        assert_eq!(normalize_page_url("/dpe"), "other");
    }

    #[test]
    fn unrelated_paths_return_other() {
        assert_eq!(normalize_page_url("/admin/secret"), "other");
        assert_eq!(normalize_page_url("/dpe/oai"), "other");
        assert_eq!(normalize_page_url("/dpe/api/v2/projects"), "other");
    }
}
