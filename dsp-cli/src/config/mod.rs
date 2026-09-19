//! Config resolution — layer 5 of ADR-0008.
//!
//! The four-layer stack from ADR-0007: flag → env var → `.env` (CWD) →
//! fail. `.env` is loaded via `dotenvy::dotenv()` at startup; no
//! hard-coded default server.
//!
//! By the time [`Config::resolve`] is called, the chain has already collapsed
//! to a single `Option<&str>`: clap's `env = "DSP_SERVER"` attribute merged
//! the flag and env-var tiers; `dotenvy::dotenv()` in `main` populated the
//! env from `.env` before clap ran. This function handles only the final
//! two cases: expand a shortcut or literal, or fail.

pub mod auth_cache;
pub use auth_cache::AuthCache;

pub mod token;
pub use token::{ResolvedToken, TokenOrigin, resolve_token};

use crate::diagnostic::Diagnostic;

/// Built-in shortcut names → canonical server URLs. See ADR-0007.
///
/// Lookup is linear; a handful of entries is too small to justify a `HashMap`.
/// Matching is case-insensitive (`PROD`, `Prod`, `prod` all resolve); a value
/// that matches no shortcut passes through unchanged as a literal URL.
///
/// Reachability of each host was last verified 2026-05-27 via `GET /health`.
const SHORTCUTS: &[(&str, &str)] = &[
    ("prod", "https://api.dasch.swiss"),
    ("stage", "https://api.stage.dasch.swiss"),
    ("dev", "https://api.dev.dasch.swiss"),
    ("demo", "https://api.demo.dasch.swiss"),
    ("rdu", "https://api.rdu.dasch.swiss"),
    ("ls-prod", "https://api.ls-prod-server.dasch.swiss"),
    ("ls-test", "https://api.ls-test-server.dasch.swiss"),
    ("local", "http://0.0.0.0:3333"),
];

/// Resolved configuration for a single command invocation.
#[derive(Debug, Clone)]
pub struct Config {
    /// The fully-resolved server URL (or shortcut-expanded URL).
    pub server: String,
}

impl Config {
    /// Resolve the active server from the collapsed `Option<&str>`.
    ///
    /// `server` is the value after clap has merged `--server` and
    /// `DSP_SERVER` (including any `.env` values dotenvy loaded at startup).
    /// `None` means the user provided nothing — that's a usage error.
    pub fn resolve(server: Option<&str>) -> Result<Self, Diagnostic> {
        match server {
            None => Err(Diagnostic::Usage(
                "no server specified. Provide one via --server <prod|dev|…|URL>, \
the DSP_SERVER environment variable, or a .env file in the current directory. \
See `dsp docs connecting` for details."
                    .to_string(),
            )),
            Some(s) => {
                // Case-insensitive shortcut match; lowercase only for the lookup
                // so a literal URL passes through with its original casing intact.
                let lower = s.to_ascii_lowercase();
                let url = SHORTCUTS
                    .iter()
                    .find(|(name, _)| *name == lower)
                    .map(|(_, url)| *url)
                    .unwrap_or(s);

                tracing::debug!(server = url, "resolved server");

                Ok(Config {
                    server: url.to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Diagnostic;

    #[test]
    fn resolve_with_literal_url() {
        let cfg = Config::resolve(Some("https://api.example.org")).unwrap();
        assert_eq!(cfg.server, "https://api.example.org");
    }

    #[test]
    fn resolve_with_known_shortcut_prod() {
        let cfg = Config::resolve(Some("prod")).unwrap();
        assert_eq!(cfg.server, "https://api.dasch.swiss");
    }

    #[test]
    fn resolve_with_known_shortcut_local() {
        let cfg = Config::resolve(Some("local")).unwrap();
        assert_eq!(cfg.server, "http://0.0.0.0:3333");
    }

    #[test]
    fn resolve_with_unknown_word_passes_through() {
        let cfg = Config::resolve(Some("staging-experiment")).unwrap();
        assert_eq!(cfg.server, "staging-experiment");
    }

    #[test]
    fn resolve_with_known_shortcut_dev() {
        let cfg = Config::resolve(Some("dev")).unwrap();
        assert_eq!(cfg.server, "https://api.dev.dasch.swiss");
    }

    #[test]
    fn resolve_with_known_shortcut_demo() {
        let cfg = Config::resolve(Some("demo")).unwrap();
        assert_eq!(cfg.server, "https://api.demo.dasch.swiss");
    }

    #[test]
    fn resolve_shortcut_is_case_insensitive() {
        // Mixed and upper case both resolve to the canonical URL.
        assert_eq!(
            Config::resolve(Some("PROD")).unwrap().server,
            "https://api.dasch.swiss"
        );
        assert_eq!(
            Config::resolve(Some("Dev")).unwrap().server,
            "https://api.dev.dasch.swiss"
        );
    }

    #[test]
    fn resolve_literal_url_preserves_case() {
        // A non-shortcut value passes through unchanged — casing is NOT lowered.
        let cfg = Config::resolve(Some("https://API.Example.ORG/Path")).unwrap();
        assert_eq!(cfg.server, "https://API.Example.ORG/Path");
    }

    #[test]
    fn resolve_with_none_returns_usage_diagnostic() {
        let err = Config::resolve(None).unwrap_err();
        assert!(matches!(err, Diagnostic::Usage(_)));
    }

    #[test]
    fn missing_server_message_mentions_all_three_paths() {
        let err = Config::resolve(None).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("--server"), "missing --server in: {msg}");
        assert!(msg.contains("DSP_SERVER"), "missing DSP_SERVER in: {msg}");
        assert!(msg.contains(".env"), "missing .env in: {msg}");
    }

    #[test]
    fn shortcut_and_canonical_url_resolve_identically() {
        // set_entry(server, …) and token(server) both use the resolved URL as the map
        // key, so "dev" and its canonical expansion must produce the same string.
        let via_shortcut = Config::resolve(Some("dev")).unwrap();
        let via_url = Config::resolve(Some("https://api.dev.dasch.swiss")).unwrap();
        assert_eq!(
            via_shortcut.server, via_url.server,
            "shortcut 'dev' and its URL must resolve to the same string for \
cache key lookups to work"
        );
    }
}
