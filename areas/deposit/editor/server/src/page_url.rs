//! The page-URL normalizer passed into `shared_telemetry::collector::collect_route`:
//! identity's ([`identity_web::normalize_page_url`]) first, then the editor's
//! ([`editor_web::normalize_page_url`]), each owning its own routes.
//!
//! Bounds the `page.url` metric attribute to the service's own routes so browser
//! metrics stay breakable down by page without exploding cardinality on
//! arbitrary paths. Lives here rather than in `shared-telemetry` because a
//! shared crate cannot know one service's route table
//! (`docs/src/repo_structure.md` → Shared Crates).
//!
//! A path neither capability labels is `other`. Both capabilities strip a query
//! string before matching, so the composition survives the beacon one day
//! sending `location.href` rather than today's `location.pathname`.

/// Normalize a page URL to a known route pattern, identity's or the editor's.
/// Returns "other" for unrecognized paths to prevent metric cardinality explosion.
pub fn normalize_page_url(url: &str) -> &'static str {
    identity_web::normalize_page_url(url)
        .or_else(|| editor_web::normalize_page_url(url))
        .unwrap_or("other")
}

#[cfg(test)]
mod tests {

    /// The row actions emit their own beacon, so without an entry they land in
    /// `"other"` beside genuinely unknown paths.
    #[test]
    fn the_row_action_routes_are_attributed_rather_than_bucketed_as_other() {
        assert_eq!(
            normalize_page_url("/projects/0801d/sections/dataset/fields/keywords/add"),
            "/projects/{shortcode}/sections/{section}/fields/{field}/add"
        );
        assert_eq!(
            normalize_page_url("/projects/080C/sections/dataset/fields/keywords/r0/remove"),
            "/projects/{shortcode}/sections/{section}/fields/{field}/{key}/remove"
        );
        // Neither the field id nor the row key may reach the attribute verbatim.
        for url in [
            "/projects/0801d/sections/dataset/fields/keywords/add",
            "/projects/0801d/sections/dataset/fields/attributions/r7/remove",
        ] {
            let normalized = normalize_page_url(url);
            assert!(!normalized.contains("keywords"), "{normalized}");
            assert!(!normalized.contains("attributions"), "{normalized}");
            assert!(!normalized.contains("r7"), "{normalized}");
        }
        // Still "other" for a shape that is not one of the two.
        assert_eq!(normalize_page_url("/projects/0801d/sections/dataset/fields/keywords"), "other");
        assert_eq!(normalize_page_url("/projects/0801d/sections/dataset/nonsense/x/add"), "other");
    }
    /// The entity form and its row actions each emit a beacon, and the proposal
    /// segment is an unbounded `entity_id`.
    #[test]
    fn the_entity_form_routes_are_attributed_and_their_segments_collapse() {
        assert_eq!(
            normalize_page_url("/projects/0801d/entities/person-417"),
            "/projects/{shortcode}/entities/{proposal}"
        );
        assert_eq!(
            normalize_page_url("/projects/0801d/entities/person-417/fields/givenNames/add"),
            "/projects/{shortcode}/entities/{proposal}/fields/{field}/add"
        );
        assert_eq!(
            normalize_page_url("/projects/080C/entities/organization-143/fields/sameAs/r2/remove"),
            "/projects/{shortcode}/entities/{proposal}/fields/{field}/{key}/remove"
        );
        // The shortcode, the entity id, the field id and the row key are all
        // unbounded, so none of them may reach the attribute verbatim.
        for url in [
            "/projects/080C/entities/person-417",
            "/projects/080C/entities/person-417/fields/givenNames/add",
            "/projects/080C/entities/organization-143/fields/sameAs/r2/remove",
        ] {
            let normalized = normalize_page_url(url);
            for leaked in ["080C", "person-417", "organization-143", "givenNames", "sameAs", "r2"] {
                assert!(!normalized.contains(leaked), "{url} leaked {leaked} as {normalized}");
            }
        }
        // A shape that is none of the three stays "other".
        assert_eq!(normalize_page_url("/projects/0801d/entities"), "other");
        assert_eq!(normalize_page_url("/projects/0801d/entities/"), "other");
        assert_eq!(normalize_page_url("/projects/0801d/entities/person-417/nonsense"), "other");
        assert_eq!(normalize_page_url("/projects/0801d/entities/person-417/fields/x"), "other");
    }

    use super::*;

    #[test]
    fn known_routes() {
        assert_eq!(normalize_page_url("/login"), "/login");
        assert_eq!(normalize_page_url("/login/code"), "/login/code");
        assert_eq!(normalize_page_url("/projects"), "/projects");
        assert_eq!(normalize_page_url("/states"), "/states");
        assert_eq!(normalize_page_url("/review"), "/review");
        assert_eq!(normalize_page_url("/collection"), "/collection");
        assert_eq!(normalize_page_url("/depositors"), "/depositors");
        assert_eq!(normalize_page_url("/depositors/new"), "/depositors/new");
    }

    #[test]
    fn variable_segments_collapse_to_their_pattern() {
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
        assert_eq!(
            normalize_page_url(&format!("/collection/{id}/discard")),
            "/collection/{id}/discard"
        );
    }

    #[test]
    fn the_root_is_a_redirect_and_so_not_a_page() {
        // It renders no beacon script, so no beacon can report it.
        assert_eq!(normalize_page_url("/"), "other");
    }

    #[test]
    fn unrelated_paths_return_other() {
        assert_eq!(normalize_page_url("/healthz"), "other");
        assert_eq!(normalize_page_url("/admin/secret"), "other");
        // Near misses stay bounded: a trailing segment does not mint a new
        // attribute value.
        assert_eq!(normalize_page_url("/login/code/extra"), "other");
        // Not a page: the edit form posts to `/depositors/{id}/edit`, so nothing
        // renders here.
        assert_eq!(normalize_page_url("/depositors/0c1cd9ff-9a9f-4b0e"), "other");
        assert_eq!(normalize_page_url("/projects/"), "other");
        // A redirect into the first section, so nothing renders and no beacon
        // can report it.
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
        assert_eq!(normalize_page_url("/collection/abc/delete"), "other");
        assert_eq!(normalize_page_url("/collection//discard"), "other");
        assert_eq!(normalize_page_url("/collection/"), "other");
    }

    #[test]
    fn a_query_is_stripped_rather_than_collapsing_the_page_into_other() {
        assert_eq!(normalize_page_url("/review/0801d?show=all"), "/review/{shortcode}");
        // `telemetry.js` sends `location.pathname`, so nothing arrives with a
        // query today; stripping is what survives it one day sending
        // `location.href`.
        assert_eq!(normalize_page_url("/login?next=/projects"), "/login");
        assert_eq!(
            normalize_page_url("/projects/0801/sections/overview?x=1"),
            "/projects/{shortcode}/sections/{section}"
        );
        assert_eq!(normalize_page_url("/projects#section"), "/projects");
        assert_eq!(normalize_page_url("/nope?x=1"), "other");
    }
}
