//! OAI-PMH 2.0 Data Provider implementation.
//!
//! This crate implements the OAI-PMH 2.0 protocol for exposing Research Projects
//! and Project Clusters as harvestable metadata records.

mod error;
mod handlers;
mod metadata;
mod resumption;
mod xml;

pub use handlers::oai_handler;
pub use metadata::project_oai_identifier;

/// Fallback OAI-PMH base URL when [`OaiState::new`] is given an empty string.
/// The production canonical endpoint; mirrors the `DpeConfig::oai_base_url` default.
const DEFAULT_BASE_URL: &str = "https://repository.dasch.swiss/dpe/oai";

/// The OAI-PMH handler's state: the public base URL emitted as `baseURL` and in
/// `<request>` elements. Built once by the composition root and handed to
/// `oai_handler` through axum's `State` extractor — never a process-global.
#[derive(Clone)]
pub struct OaiState {
    pub base_url: String,
}

impl OaiState {
    /// Applies the empty-to-default and trailing-slash normalisation exactly
    /// once, at construction.
    pub fn new(configured: &str) -> Self {
        Self {
            base_url: resolve_url(Some(configured.to_string()), DEFAULT_BASE_URL),
        }
    }
}

/// Strips any trailing slash so callers can concatenate a path unconditionally.
fn resolve_url(explicit: Option<String>, default: &str) -> String {
    explicit
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod oai_state_tests {
    use super::{OaiState, DEFAULT_BASE_URL};

    #[test]
    fn explicit_value_is_used() {
        assert_eq!(
            OaiState::new("https://api.dev.dasch.swiss/dpe/oai").base_url,
            "https://api.dev.dasch.swiss/dpe/oai"
        );
    }

    #[test]
    fn empty_falls_back_to_default() {
        assert_eq!(OaiState::new("").base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn default_is_the_production_endpoint_not_meta() {
        assert_eq!(DEFAULT_BASE_URL, "https://repository.dasch.swiss/dpe/oai");
        assert!(!DEFAULT_BASE_URL.contains("meta.dasch.swiss"));
    }

    #[test]
    fn trailing_slash_is_stripped() {
        assert_eq!(
            OaiState::new("https://repository.dasch.swiss/dpe/oai/").base_url,
            "https://repository.dasch.swiss/dpe/oai"
        );
    }
}
