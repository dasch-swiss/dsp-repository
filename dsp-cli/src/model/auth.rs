//! Authentication domain shapes — shared across the client → action boundary.
//!
//! Types here are the dsp-cli vocabulary for auth data. DSP-API wire types
//! (e.g. `LoginApiResponse`) live inside `src/client/http.rs` and are never
//! exposed above the client layer. See ADR-0001 and ADR-0008.

/// The result of a successful login call, expressed in dsp-cli vocabulary.
///
/// `Clone` is derived so `MockDspClient` can hand out `Result<LoginResponse, Diagnostic>`
/// values repeatedly without consuming them.
#[derive(Debug, Clone)]
pub struct LoginResponse {
    pub token: String,
    pub user: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}
