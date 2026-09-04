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
//! its own hostname. `/projects/{shortcode}/sections/{section}` has joined it, and
//! `/projects/{shortcode}` **left** at the same moment: the scheme in
//! `architecture.md` turned it into a redirect to the first section, and a
//! redirect renders no beacon script, so no beacon can report it.
//!
//! `/` is deliberately absent. It is a redirect, so it never renders the beacon
//! script and no beacon can report it — the same reason `/` and `/dpe` are
//! absent from DPE's list. The normalizer still answers sanely if one somehow
//! did.
//!
//! A query string is stripped before matching. The beacon reports
//! `location.pathname` today, so nothing arrives with one — but `/login?next=…`
//! exists now, and an earlier version of this module relied on the beacon's
//! shape instead of handling the query, with a test that asserted the *broken*
//! outcome. Stripping is what actually survives the beacon one day sending
//! `location.href`; asserting that it currently does not is not a guard.
//!
//! DPE's normalizer does not strip, and does not need to: it has no route that
//! navigates by query.
//!
//! REVIEW: new full-page routes in `router.rs` need a matching entry here —
//! see `REVIEW.md`.
const KNOWN_ROUTES: &[&str] = &[
    "/login",
    "/login/code",
    "/projects",
    "/review",
    "/depositors",
    "/depositors/new",
];

/// Normalize a page URL to a known editor route pattern.
/// Returns "other" for unrecognized paths to prevent metric cardinality explosion.
pub fn normalize_page_url(url: &str) -> &'static str {
    // Everything below matches on a whole path, so a query or a fragment has to
    // come off first or it turns every page view into `other`.
    let url = url.split(['?', '#']).next().unwrap_or(url);
    for &route in KNOWN_ROUTES {
        if route == url {
            return route;
        }
    }
    // Pattern matches for the routes with a variable segment, without
    // allocating — `split_once` rather than `split().collect::<Vec<_>>()`, which
    // an earlier version used while this comment claimed otherwise. A shortcode
    // and an account id are both unbounded sets, so letting either through
    // verbatim is the cardinality explosion this module exists to prevent.
    if let Some(rest) = url.strip_prefix("/projects/") {
        // Both segments collapse. A section id is a closed set and would be safe
        // to keep, but a shortcode is not, and the pair is one attribute value:
        // `/projects/0801/sections/overview` and `/projects/080C/sections/overview`
        // have to arrive as the same one.
        if let Some((shortcode, tail)) = rest.split_once('/') {
            if !shortcode.is_empty() {
                if let Some(section) = tail.strip_prefix("sections/") {
                    if !section.is_empty() && !section.contains('/') {
                        return "/projects/{shortcode}/sections/{section}";
                    }
                }
            }
        }
    }
    // The shortcode collapses, for the reason the project routes' does: it is
    // an unbounded set, and letting it through verbatim is the cardinality
    // explosion this module exists to prevent.
    if let Some(rest) = url.strip_prefix("/review/") {
        if !rest.is_empty() && !rest.contains('/') {
            return "/review/{shortcode}";
        }
        return "other";
    }
    if let Some(rest) = url.strip_prefix("/depositors/") {
        return match rest.split_once('/') {
            Some((id, "edit")) if !id.is_empty() => "/depositors/{id}/edit",
            Some((id, "remove")) if !id.is_empty() => "/depositors/{id}/remove",
            _ => "other",
        };
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
        assert_eq!(normalize_page_url("/review"), "/review");
        assert_eq!(normalize_page_url("/depositors"), "/depositors");
        assert_eq!(normalize_page_url("/depositors/new"), "/depositors/new");
    }

    #[test]
    fn variable_segments_collapse_to_their_pattern() {
        // A shortcode and an account id are both unbounded sets. Letting either
        // through verbatim is exactly the cardinality explosion this module
        // exists to prevent.
        assert_eq!(
            normalize_page_url("/projects/0801/sections/overview"),
            "/projects/{shortcode}/sections/{section}"
        );
        assert_eq!(
            normalize_page_url("/projects/080C/sections/dataset"),
            "/projects/{shortcode}/sections/{section}"
        );
        assert_eq!(normalize_page_url("/review/0801d"), "/review/{shortcode}");
        assert_eq!(normalize_page_url("/review/080C"), "/review/{shortcode}");
        let id = "0c1cd9ff-9a9f-4b0e-9b0a-3f2f8f0b7a11";
        assert_eq!(normalize_page_url(&format!("/depositors/{id}/edit")), "/depositors/{id}/edit");
        assert_eq!(
            normalize_page_url(&format!("/depositors/{id}/remove")),
            "/depositors/{id}/remove"
        );
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
        // Near misses stay bounded: a trailing segment does not mint a new
        // attribute value.
        assert_eq!(normalize_page_url("/login/code/extra"), "other");
        // No longer a page: the edit form posts to `/depositors/{id}/edit`, so
        // nothing renders here and a beacon cannot report it.
        assert_eq!(normalize_page_url("/depositors/0c1cd9ff-9a9f-4b0e"), "other");
        assert_eq!(normalize_page_url("/projects/"), "other");
        // No longer a page: it is a redirect into the first section, so it
        // renders no beacon script and nothing can report it.
        assert_eq!(normalize_page_url("/projects/0801"), "other");
        assert_eq!(normalize_page_url("/projects/0801a"), "other");
        // Near misses stay bounded rather than minting a pattern with a blank or
        // an extra segment.
        assert_eq!(normalize_page_url("/projects//sections/overview"), "other");
        assert_eq!(normalize_page_url("/projects/0801/sections/"), "other");
        assert_eq!(normalize_page_url("/projects/0801/sections/a/b"), "other");
        assert_eq!(normalize_page_url("/projects/0801/settings"), "other");
        assert_eq!(normalize_page_url("/review/"), "other");
        assert_eq!(normalize_page_url("/review/0801d/fields"), "other");
        assert_eq!(normalize_page_url("/depositors/abc/delete"), "other");
        assert_eq!(normalize_page_url("/depositors//edit"), "other");
        // An empty id is not an id: the guards keep `/depositors//…` out of
        // every pattern rather than minting one with a blank segment.
        assert_eq!(normalize_page_url("/depositors/"), "other");
    }

    #[test]
    fn a_query_is_stripped_rather_than_collapsing_the_page_into_other() {
        assert_eq!(normalize_page_url("/review/0801d?show=all"), "/review/{shortcode}");
        // `telemetry.js` sends `location.pathname`, so nothing arrives with a
        // query today. The previous version of this test asserted that fact —
        // `assert_ne!(normalize_page_url("/login?next=…"), "/login")` — and
        // called itself the guard against the beacon one day sending
        // `location.href`. It was the opposite: it pinned the broken outcome as
        // expected, and would have stayed green through exactly that change
        // while every login page view collapsed into `other`.
        assert_eq!(normalize_page_url("/login?next=/projects"), "/login");
        assert_eq!(
            normalize_page_url("/projects/0801/sections/overview?x=1"),
            "/projects/{shortcode}/sections/{section}"
        );
        assert_eq!(normalize_page_url("/projects#section"), "/projects");
        assert_eq!(normalize_page_url("/nope?x=1"), "other");
    }
}
