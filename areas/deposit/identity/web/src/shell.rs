//! `IdentityState`, and the response glue around `deposit-shell`'s pages.

/// Shared state for identity's handlers. Cheap to clone: everything heavy is behind an `Arc`.
#[derive(Clone)]
pub(crate) struct IdentityState {
    /// Unhashed in dev, content-hashed in release.
    pub(crate) css_href: String,
    /// Identity's store, behind its ports rather than [`identity_store::Database`], so a test
    /// can make a chosen storage call fail; the auth flow's error branches depend on it.
    pub(crate) store: std::sync::Arc<dyn identity_core::repository::IdentityRepositories>,
    /// The editor work an account still has open, for the depositor-removal confirmation.
    pub(crate) open_work: std::sync::Arc<dyn identity_ports::OpenWork>,
    /// Behind a trait so a test can watch what was sent and make sending fail.
    pub(crate) mailer: std::sync::Arc<dyn identity_core::mail::Mailer>,
    pub(crate) auth: identity_core::login::AuthConfig,
    /// Whether the code-entry screen may show the login code. Resolved once at startup by the
    /// root, never re-derived.
    pub(crate) reveal_login_code: bool,
}

/// Render a page inside the document shell: the one place a `Markup` becomes a `Response`.
///
/// `viewer` is the signed-in account's display name, never its address: the header is on every
/// page and in every screenshot of one.
pub(crate) fn render(
    state: &IdentityState,
    title: &str,
    status: axum::http::StatusCode,
    viewer: Option<&str>,
    content: maud::Markup,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let traceparent = crate::traceparent::extract_traceparent();
    let viewer = viewer.map(|name| deposit_shell::view::Viewer { name });
    let body = deposit_shell::view::page(title, traceparent.as_deref(), &state.css_href, viewer, content);
    (status, axum::response::Html(body.into_string())).into_response()
}

/// A 403 rendered as a page, so it is not a dead end. Only authenticated requests reach it,
/// so the header renders signed in.
pub(crate) fn forbidden(state: &IdentityState, viewer: &str, message: &str) -> axum::response::Response {
    render(
        state,
        "No access — DaSCH Metadata Editor",
        axum::http::StatusCode::FORBIDDEN,
        Some(viewer),
        deposit_shell::pages::forbidden::forbidden(message),
    )
}

/// A 404 for an account id that names nothing. Renders signed out, as the root's fallback does.
pub(crate) fn not_found(state: &IdentityState) -> axum::response::Response {
    render(
        state,
        "Page not found — DaSCH Metadata Editor",
        axum::http::StatusCode::NOT_FOUND,
        None,
        deposit_shell::pages::not_found::not_found(),
    )
}

/// An instant as a page shows it: UTC, and labelled so.
pub(crate) fn format_instant(at: chrono::DateTime<chrono::Utc>) -> String {
    at.format("%Y-%m-%d %H:%M UTC").to_string()
}
