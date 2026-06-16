//! OAI-PMH 2.0 Data Provider implementation.
//!
//! This crate implements the OAI-PMH 2.0 protocol for exposing Research Projects
//! and Project Clusters as harvestable metadata records.

mod error;
mod handlers;
mod metadata;
mod xml;

use std::sync::OnceLock;

pub use handlers::oai_handler;

/// Fallback OAI-PMH base URL when neither [`set_base_url`] nor `DPE_OAI_BASE_URL` is set.
/// The production canonical endpoint; mirrors the `DpeConfig::oai_base_url` default.
const DEFAULT_BASE_URL: &str = "https://repository.dasch.swiss/dpe/oai";

static BASE_URL: OnceLock<String> = OnceLock::new();

/// Resolves the base URL from an optional explicit value, falling back to the default.
/// Pure helper so the precedence is unit-testable without touching the process-global.
fn resolve_base_url(explicit: Option<String>) -> String {
    explicit
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
}

/// Sets the public OAI-PMH base URL at startup. Must be called before the first request.
/// Thread-safe: uses OnceLock (first call wins, subsequent calls are no-ops), matching
/// `dpe_core::set_data_dir`.
pub fn set_base_url(url: &str) {
    if BASE_URL.set(resolve_base_url(Some(url.to_string()))).is_err() {
        tracing::warn!(new = url, current = %base_url(), "set_base_url called again but base URL is already set");
    }
}

/// The public OAI-PMH base URL emitted as `baseURL` and in `<request>` elements.
///
/// Priority: OnceLock (set by main.rs from `DpeConfig`) → `DPE_OAI_BASE_URL` env var →
/// [`DEFAULT_BASE_URL`]. The env fallback keeps the value correct in contexts that do not
/// call [`set_base_url`] (e.g. tests).
pub(crate) fn base_url() -> &'static str {
    BASE_URL.get_or_init(|| resolve_base_url(std::env::var("DPE_OAI_BASE_URL").ok()))
}

#[cfg(test)]
mod base_url_tests {
    use super::{resolve_base_url, DEFAULT_BASE_URL};

    #[test]
    fn explicit_value_is_used() {
        assert_eq!(
            resolve_base_url(Some("https://api.dev.dasch.swiss/dpe/oai".to_string())),
            "https://api.dev.dasch.swiss/dpe/oai"
        );
    }

    #[test]
    fn empty_or_absent_falls_back_to_default() {
        assert_eq!(resolve_base_url(None), DEFAULT_BASE_URL);
        assert_eq!(resolve_base_url(Some(String::new())), DEFAULT_BASE_URL);
    }

    #[test]
    fn default_is_the_production_endpoint_not_meta() {
        assert_eq!(DEFAULT_BASE_URL, "https://repository.dasch.swiss/dpe/oai");
        assert!(!DEFAULT_BASE_URL.contains("meta.dasch.swiss"));
    }
}
