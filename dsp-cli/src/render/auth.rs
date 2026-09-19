//! Auth renderer-input shapes — typed values passed to `Renderer::auth_*` methods.
//!
//! These are render-layer types, not domain model types. They live here rather
//! than in `src/model/` because they flow from the action layer into the
//! renderer, not across the client→action boundary. See ADR-0008 for the
//! layer split.

use chrono::{DateTime, Utc};

/// Outcome passed to `Renderer::auth_login` on a successful login.
pub struct AuthLoginOutcome {
    /// Resolved server URL (after `Config::resolve`).
    pub server: String,
    /// Authenticated user identifier (email, username, or IRI) — echoed from
    /// the `--user` value the user supplied.
    pub user: String,
    /// Token expiry derived from the JWT `exp` claim, if present.
    pub expires_at: Option<DateTime<Utc>>,
}

/// Outcome passed to `Renderer::auth_status` — either logged-in or not.
pub enum AuthStatusOutcome {
    /// A token for this server exists in the auth cache.
    LoggedIn {
        /// Resolved server URL.
        server: String,
        /// Username stored in the cache, if any.
        user: Option<String>,
        /// Token expiry stored in the cache, if any.
        expires_at: Option<DateTime<Utc>>,
        /// `true` when `expires_at` is set and is in the past.
        expired: bool,
    },
    /// A token was supplied via the `DSP_TOKEN` environment variable, which
    /// overrides the cached token (ADR-0007). No user is available (the cached
    /// `user` comes from the login response body, not from the token itself).
    /// Expiry is read via `client::jwt::extract_exp` for display only; if the
    /// token is not a parseable JWT, `expires_at` is `None` ("expiry unknown").
    AuthenticatedViaEnv {
        /// Resolved server URL.
        server: String,
        /// Token expiry derived from the JWT `exp` claim, if parseable.
        /// `None` means the token's `exp` was unreadable ("expiry unknown").
        expires_at: Option<DateTime<Utc>>,
        /// `true` when `expires_at` is `Some` and the timestamp is in the past.
        expired: bool,
    },
    /// No token for this server in the auth cache (or no cache file).
    NotLoggedIn {
        /// Resolved server URL.
        server: String,
    },
}

/// Outcome passed to `Renderer::auth_logout`.
pub struct AuthLogoutOutcome {
    /// Resolved server URL.
    pub server: String,
    /// `true` when an entry was actually removed from the cache.
    pub was_cached: bool,
}

/// Outcome passed to `Renderer::auth_set_token` after caching a stdin-provided token.
pub struct AuthSetTokenOutcome {
    /// Resolved server URL (after `Config::resolve`).
    pub server: String,
    /// User identifier from the JWT `sub` claim (a user IRI); `None` if the
    /// claim is absent. This is display-only metadata — it is **not** verified
    /// and must not be used as an access-control input.
    pub user: Option<String>,
    /// Token expiry derived from the JWT `exp` claim, if present.
    pub expires_at: Option<DateTime<Utc>>,
}
