//! Layer-neutral utility helpers.
//!
//! This module is the home for dependency-free helpers that may be imported
//! from any layer (`client`, `render`, …) without creating a cross-layer
//! dependency (ADR-0008). Unlike `src/client/` or `src/render/`, `src/util/`
//! imports from no other `dsp-cli` layer; all layers may import from it.

pub(crate) mod text;

/// The `User-Agent` header value sent on every outgoing HTTP request
/// (DSP-API calls and the crates.io update check alike), so server operators
/// can identify dsp-cli traffic. Plain form `dsp-cli/<version>`; the version is
/// baked in at compile time from Cargo. See plan 033.
pub(crate) const USER_AGENT: &str = concat!("dsp-cli/", env!("CARGO_PKG_VERSION"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_has_expected_prefix() {
        assert!(USER_AGENT.starts_with("dsp-cli/"));
    }

    #[test]
    fn user_agent_is_plain_form_no_suffix() {
        assert_eq!(USER_AGENT, concat!("dsp-cli/", env!("CARGO_PKG_VERSION")));
    }
}
