//! The service's page-URL normalizer, passed into
//! `shared_telemetry::collector::collect_route`.
//!
//! Bounds the `page.url` metric attribute to the editor's own routes so browser
//! metrics stay breakable down by page without exploding cardinality on
//! arbitrary paths. Lives here rather than in `shared-telemetry` because a
//! shared crate cannot know one service's route table
//! (`docs/src/repo_structure.md` → Shared Crates).
//!
//! The label is the template of the route axum would match, built from the
//! editor's ([`editor_web::route_paths`]) and identity's
//! ([`identity_web::route_paths`]) route tables with `matchit`, the path router
//! axum itself matches with. A route is labelled as
//! soon as it is declared, so there is no second list to keep in step, and a
//! variable segment (a shortcode, an account or entity id: unbounded sets)
//! never reaches the attribute verbatim. A path no
//! route matches is `other`. The redirects `/` and `/projects/{shortcode}` get
//! labels too, though neither renders the beacon script.
//!
//! A query string is stripped before matching, so the normalizer survives the
//! beacon one day sending `location.href` rather than today's
//! `location.pathname`.
use std::collections::BTreeSet;
use std::sync::LazyLock;

/// The paths of every traced route, the editor's and identity's.
///
/// Deduplicated: two entries may share a path with different methods, which
/// axum merges and `matchit` would refuse as a conflict.
fn route_paths() -> BTreeSet<&'static str> {
    editor_web::route_paths().chain(identity_web::route_paths()).collect()
}

/// Every traced route's template, keyed by itself.
static ROUTES: LazyLock<matchit::Router<&'static str>> = LazyLock::new(|| {
    let mut router = matchit::Router::new();
    for path in route_paths() {
        // A real conflict also panics axum when it builds the router, so a
        // binary that started cannot reach this.
        router.insert(path, path).expect("axum accepted the same route table");
    }
    router
});

/// Normalize a page URL to the template of the editor or identity route it matches.
/// Returns "other" for unrecognized paths to prevent metric cardinality explosion.
pub fn normalize_page_url(url: &str) -> &'static str {
    // A query or a fragment has to come off first or it turns every page view
    // into `other`.
    let url = url.split(['?', '#']).next().unwrap_or(url);
    ROUTES.at(url).map_or("other", |matched| *matched.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_is_stripped_rather_than_collapsing_the_page_into_other() {
        assert_eq!(normalize_page_url("/review/0801d?show=all"), "/review/{shortcode}");
        // `telemetry.js` sends `location.pathname`, so nothing arrives with a
        // query today; stripping is what survives it one day sending
        // `location.href`.
        assert_eq!(normalize_page_url("/login?next=/projects"), "/login");
        assert_eq!(normalize_page_url("/projects#section"), "/projects");
        assert_eq!(normalize_page_url("/nope?x=1"), "other");
    }

    /// The label is only as good as `matchit` agreeing with axum. Should axum
    /// move to another `matchit` than the workspace pins, this fails rather than
    /// the labels drifting from the routes that serve the pages.
    #[tokio::test]
    async fn every_label_is_the_route_axum_matches() {
        use axum::body::Body;
        use axum::extract::MatchedPath;
        use axum::http::Request;
        use tower::ServiceExt;

        let app = route_paths()
            .into_iter()
            .fold(axum::Router::new(), |router, path| {
                router.route(
                    path,
                    axum::routing::any(|matched: MatchedPath| async move { matched.as_str().to_owned() }),
                )
            })
            .fallback(|| async { "other" });
        for url in [
            "/",
            "/login",
            "/login/code",
            "/depositors/new",
            "/depositors/0c1cd9ff-9a9f-4b0e-9b0a-3f2f8f0b7a11/edit",
            "/projects/080C",
            "/projects/080C/sections/overview",
            "/projects//sections/overview",
            "/projects/080C/sections/dataset/fields/keywords/r0/remove",
            "/projects/080C/entities/person-417/fields/givenNames/add",
            "/depositors//edit",
            "/projects/x/sections/y/fields/a/b/c/add",
            "/projects/",
            "/nope",
        ] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(url).body(Body::empty()).unwrap())
                .await
                .unwrap();
            let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert_eq!(normalize_page_url(url), std::str::from_utf8(&body).unwrap(), "{url}");
        }
    }
}
