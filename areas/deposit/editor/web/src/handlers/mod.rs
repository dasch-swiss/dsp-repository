//! The editor's handlers. Only these modules use axum; the views in `pages/`, `form/` and
//! `entity/` do not.

use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;

use crate::handlers::auth::Account;
use crate::shell::EditorState;

pub(crate) mod approved;
pub(crate) mod auth;
pub(crate) mod entities;
pub(crate) mod projects;
pub(crate) mod review;
pub(crate) mod sections;

/// The header the vendored Datastar bundle sets on every fetch it makes.
///
/// Presence is the whole test: the bundle sends the literal `true`, and a
/// request that carries the name at all came from it. A hand-built request can
/// set it too, which changes nothing that matters — both renderings are the same
/// data, and neither is a permission.
pub(crate) const DATASTAR_REQUEST: &str = "datastar-request";

/// Whether this request came from the Datastar bundle.
pub(crate) fn is_enhanced(headers: &HeaderMap) -> bool {
    headers.contains_key(DATASTAR_REQUEST)
}

/// What a surface calls itself when storage does not answer: the full log
/// message, and the noun the problem page uses ("page" or "form").
pub(crate) struct StorageSurface {
    pub(crate) log: &'static str,
    pub(crate) noun: &'static str,
}

/// Storage would not answer, so the surface cannot show what it should.
pub(crate) fn storage_error(
    state: &EditorState,
    user: &Account,
    surface: StorageSurface,
    what: &str,
    error: &impl std::fmt::Display,
) -> Response {
    tracing::error!(error = %error, operation = what, "{}", surface.log);
    crate::shell::render(
        state,
        "Page unavailable — DaSCH Metadata Editor",
        StatusCode::INTERNAL_SERVER_ERROR,
        Some(&user.actor.display_name),
        deposit_shell::pages::problem::unavailable(&format!(
            "The editor could not reach its database, so this {} is not showing what it should. Try again; if it \
             keeps happening, the service needs attention.",
            surface.noun
        )),
    )
}
