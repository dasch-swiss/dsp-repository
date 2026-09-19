//! The [`SparqlResponse`] relay type for [`crate::client::DspClient::sparql_query`].
//!
//! Deliberately not in `src/model/` — every `src/model/` type is a
//! *translated domain* type per dsp-cli/ADR-0008's layer-4 contract, and this is
//! transport-shaped (an HTTP status and a MIME string), not a parsed model.
//! See dsp-cli/ADR-0016.

/// One relayed SPARQL response: the triplestore's own status, media type and bytes.
///
/// Deliberately not a parsed model — dsp-cli/ADR-0016. The body is whatever the store
/// serialized, in whatever format it negotiated, and dsp-cli does not interpret it.
#[derive(Clone)]
pub struct SparqlResponse {
    /// The triplestore's HTTP status, relayed by DSP-API.
    pub status: u16,
    /// The triplestore's `Content-Type`, if it sent one. Never written to stdout —
    /// it feeds the `-v` disclosure and the classifier's body-parse decision.
    pub content_type: Option<String>,
    /// The response body, byte-exact.
    pub body: Vec<u8>,
}

/// Hand-written: a derived `Debug` on a struct holding up to 64 MiB of
/// store-authored bytes would mean the first `tracing::debug!(?resp)` or the
/// codebase's standard `assert!(…, "{result:?}")` test idiom dumps
/// unsanitised store output into stderr or CI logs. Prints the body length
/// instead of its bytes, and caps `content_type` defensively.
impl std::fmt::Debug for SparqlResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content_type = self.content_type.as_deref().map(|ct| {
            let capped: String = ct.chars().take(200).collect();
            if capped.len() < ct.len() {
                format!("{capped}…")
            } else {
                capped
            }
        });
        f.debug_struct("SparqlResponse")
            .field("status", &self.status)
            .field("content_type", &content_type)
            .field("body", &format!("<{} bytes>", self.body.len()))
            .finish()
    }
}
