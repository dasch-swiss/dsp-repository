//! Every route that changes state is `POST`: the root's `Sec-Fetch-Site` control exempts `GET` and
//! `HEAD` by necessity, because a navigation from anywhere is a `GET`, so a `GET` that changes
//! state is a `GET` nothing protects. Rust cannot check the converse (a `MethodRouter` cannot be
//! asked which methods it serves), so the assertions live with the handlers, and the one deliberate
//! exception is the session's idle-timeout touch, which identity owns.
//!
//! The editor runs on its own hostname, so paths are root-mounted: there is no `/editor`
//! prefix. A shared origin would defeat the `Sec-Fetch-Site` CSRF control. See
//! `docs/src/editor/routing.md`.

use axum::routing::MethodRouter;
use shared_rate_limit::RightmostXffKeyExtractor;

use crate::shell::EditorState;

/// Per-IP budget for `GET /api/v1/approved-records`.
///
/// Sized for a CI poller checking in every few minutes, not a browser beacon
/// firing on every page: a low sustained rate with a small burst to absorb a
/// poller's retry without opening the door to a scrape of the whole table on
/// a tight loop.
const APPROVED_RECORDS_BURST: u32 = 5;
const APPROVED_RECORDS_REPLENISH_SECS: u64 = 30;

/// A per-IP rate limit keyed by [`RightmostXffKeyExtractor`].
///
/// A macro rather than a function because the layer's type parameters come from
/// the `governor` crate, which `tower_governor` re-exports only in part — naming
/// the return type would mean a direct dependency on `governor` for a
/// signature.
///
/// `replenish_secs` is the interval at which one unit of the burst comes back,
/// so the sustained rate is `1 / replenish_secs` per second per IP.
macro_rules! per_ip_limit {
    ($replenish_secs:expr, $burst:expr, $what:literal) => {{
        use tower_governor::governor::GovernorConfigBuilder;
        use tower_governor::GovernorLayer;

        let config = GovernorConfigBuilder::default()
            .per_second($replenish_secs)
            .burst_size($burst)
            .key_extractor(RightmostXffKeyExtractor)
            .finish()
            .expect(concat!($what, " GovernorConfig should build from its constants"));
        GovernorLayer { config: std::sync::Arc::new(config) }
    }};
}

pub(crate) type Route = (&'static str, MethodRouter<EditorState>);

/// The editor's routes, declared once. [`crate::Editor::router`] folds them, and [`route_paths`]
/// hands their paths to the root's page-URL normalizer, so a route declared here is labelled with
/// its own template. The root's untraced `/healthz` and `/telemetry/collect` are not, and render no
/// page.
pub(crate) fn routes() -> Vec<Route> {
    use axum::routing::{get, post};

    vec![
        // The service root: a redirect to `/projects`. The shell's header links
        // here from every page, so without it the 404 page's own header is
        // another 404.
        ("/", get(crate::shell::root)),
        // Public and unauthenticated on purpose — `crate::handlers::approved::list` takes no
        // `Authenticated`/`Rdu` extractor, which is what "public" looks like from a handler
        // signature. Traced, unlike `/telemetry/collect`: this is the one thing a CI poller
        // actually needs a span for.
        (
            "/api/v1/approved-records",
            get(crate::handlers::approved::list).layer(per_ip_limit!(
                APPROVED_RECORDS_REPLENISH_SECS,
                APPROVED_RECORDS_BURST,
                "approved records"
            )),
        ),
        // Authenticated routes
        // Every handler below takes `Authenticated`, which is what performs the
        // check: a handler that omits it is public, visibly, in its signature.
        // They take [`crate::handlers::auth`]'s pair.
        ("/projects", get(crate::handlers::projects::list)),
        ("/states", get(crate::handlers::projects::states)),
        // A redirect to the first section. Sections are real URLs rather than
        // fragment swaps, so the form is bookmarkable and Back-friendly.
        ("/projects/{shortcode}", get(crate::handlers::projects::detail)),
        // The form. One URL for the section and the save, so a rejected save
        // re-renders at a path that still answers `GET`.
        (
            "/projects/{shortcode}/sections/{section}",
            get(crate::handlers::sections::show).post(crate::handlers::sections::act),
        ),
        // Adding and removing a row of a repeatable field. Under the section's
        // own URL, and handled by the same module, so a row action resolves
        // through one `context()` — the audience gate, the lock check and the
        // shortcode fold are the section's rather than a second copy of the rule
        // that decides who may write what.
        (
            "/projects/{shortcode}/sections/{section}/fields/{field}/add",
            post(crate::handlers::sections::add_row),
        ),
        (
            "/projects/{shortcode}/sections/{section}/fields/{field}/{key}/remove",
            post(crate::handlers::sections::remove_row),
        ),
        // The entity form: one person or organisation proposal. `{proposal}` is the
        // proposal's `entity_id`, the same value `pages::section` already links to — see
        // `entities.rs`'s module docs for why. One `GET`/`POST` pair, like the section
        // form's.
        (
            "/projects/{shortcode}/entities/{proposal}",
            get(crate::handlers::entities::show).post(crate::handlers::entities::act),
        ),
        (
            "/projects/{shortcode}/entities/{proposal}/fields/{field}/add",
            post(crate::handlers::entities::add_row),
        ),
        (
            "/projects/{shortcode}/entities/{proposal}/fields/{field}/{key}/remove",
            post(crate::handlers::entities::remove_row),
        ),
        // RDU-only routes
        // `Rdu` composes `Authenticated`, so these are closed twice over: no
        // session redirects to login, and a depositor's session gets the 403
        // page. Each write shares a URL with the `GET` that renders its form, so
        // a rejected submission re-renders somewhere bookmarkable rather than on
        // a bare 405.
        //
        // The review surfaces carry claim, save and accept-all as an `intent`
        // pair on one `POST` URL per submission, for the same reason.
        ("/review", get(crate::handlers::review::queue)),
        (
            "/review/{shortcode}",
            get(crate::handlers::review::show).post(crate::handlers::review::act),
        ),
        ("/approved", get(crate::handlers::approved::overview)),
        (
            "/approved/{id}/discard",
            get(crate::handlers::approved::discard_form).post(crate::handlers::approved::discard),
        ),
    ]
}

/// The paths of the editor's routes, so the root can label a page view with its route's template
/// without seeing `EditorState`. A path may repeat.
pub fn route_paths() -> impl Iterator<Item = &'static str> {
    routes().into_iter().map(|(path, _)| path)
}
