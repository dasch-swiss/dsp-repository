//! The deployment's own ARK resolver, mounted only when
//! `DPE_ARK_RESOLVER_BASE_URL` is set.
//!
//! Production does not serve this route: there, `ark.dasch.swiss` is the
//! resolver and DPE is what it redirects to. A deployment that publishes ARKs
//! naming *itself* has to answer them, or it publishes an identifier that
//! dereferences to nothing — which is the defect this whole variable exists to
//! remove, moved one hop along.
//!
//! It is a redirect and stays one (ADR-0005, *the resolver stays a plain
//! redirect*): everything a machine reads happens in the landing page's
//! response, not here.

use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::shell::AppState;

/// The one route this module serves.
///
/// The NAAN and shoulder are literal rather than captured: `72163/1` is DaSCH's
/// and nothing else resolves here, so a request naming another authority does
/// not match the route at all and falls through to the site's 404. A test pins
/// the literal against `shared_metadata::ARK_PATH_PREFIX`.
pub(crate) const RESOLVER_ROUTE: &str = "/ark:/72163/1/{shortcode}";

/// Redirects a project ARK to that project's landing page.
///
/// `302`, which is what `ark.dasch.swiss` answers with for a project ARK
/// (verified 2026-09-18). It also happens to be the right cache semantics for
/// this route's only user: a preview is ephemeral, and a permanent redirect to
/// one would outlive it in a client's cache.
///
/// The target is built from the *resolved* project's shortcode and the
/// configured public base URL, never from the path segment — the same rule the
/// landing page's own identifiers follow (`metadata.rs`).
///
/// A shortcode no project answers to gets a `404`. The landing page answers
/// such a request with an always-200 "Project Not Found" body, and that is
/// deliberately not mirrored: there is no page here to render, and no canonical
/// URL to redirect to.
pub(crate) async fn resolve_project(State(state): State<AppState>, Path(shortcode): Path<String>) -> Response {
    let Some(raw) = dpe_core::project_cache::project_raw_by_shortcode(&shortcode) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let target = format!("{}/dpe/projects/{}", state.public_base_url, raw.shortcode);
    // `Redirect` would panic on a value the HTTP layer refuses, which a
    // configured base URL carrying a control character would be. The landing
    // page degrades rather than panicking for the same input; so does this.
    match HeaderValue::from_str(&target) {
        Ok(location) => (StatusCode::FOUND, [(header::LOCATION, location)]).into_response(),
        Err(error) => {
            tracing::warn!(shortcode = %raw.shortcode, %error, "ARK redirect target rejected by the HTTP layer");
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_route_is_the_recorded_ark_path() {
        // The ARKs the corpus records and the route that answers them have to
        // be the same string, or the resolver answers nothing it publishes.
        assert_eq!(RESOLVER_ROUTE, format!("/{}{{shortcode}}", shared_metadata::ARK_PATH_PREFIX));
    }
}
