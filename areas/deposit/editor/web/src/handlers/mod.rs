//! The editor's handlers. Only these modules use axum; the views in `pages/`, `form/` and
//! `entity/` do not.

use axum::http::HeaderMap;

pub(crate) mod auth;
pub(crate) mod collection;
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
