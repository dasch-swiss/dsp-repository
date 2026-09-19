//! Config resolution — layer 5 of dsp-cli/ADR-0008.
//!
//! The four-layer stack from dsp-cli/ADR-0007: flag → env var → `.env` (CWD) →
//! fail. `.env` is loaded via `dotenvy::dotenv()` at startup; no
//! hard-coded default server.
//!
//! By the time [`Config::resolve`] is called, the chain has already collapsed
//! to a single `Option<&str>`: clap's `env = "DSP_SERVER"` attribute merged
//! the flag and env-var tiers; `dotenvy::dotenv()` in `main` populated the
//! env from `.env` before clap ran. This function handles only the final
//! two cases: expand a shortcut or literal, or fail.
//!
//! It also validates the expanded value's scheme: a non-local `http://`
//! server is refused, because every authenticated command sends a bearer
//! token that would otherwise cross the network in cleartext. `--server
//! local` (`http://0.0.0.0:3333`) and other loopback/`localhost` addresses
//! are exempt — a bearer token never leaves the machine. `--allow-insecure-server`
//! / `DSP_ALLOW_INSECURE_SERVER` overrides the refusal.
//!
//! A value containing a control character is refused on any scheme, and that
//! refusal is not overridable.

pub mod auth_cache;
pub use auth_cache::AuthCache;

pub mod token;
pub use token::{ResolvedToken, TokenOrigin, resolve_token};

use crate::diagnostic::Diagnostic;

/// Built-in shortcut names → canonical server URLs. See dsp-cli/ADR-0007.
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
    ///
    /// `allow_insecure` is the resolved `--allow-insecure-server` /
    /// `DSP_ALLOW_INSECURE_SERVER` override (flag before env — see
    /// `Cli::allow_insecure_server` in `src/cli/mod.rs`); when `true`, the
    /// cleartext-HTTP scheme check below is skipped entirely.
    pub fn resolve(server: Option<&str>, allow_insecure: bool) -> Result<Self, Diagnostic> {
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

                // A URL never legitimately contains a raw control character, so refusing
                // outright is safer than sanitizing the stored value: sanitizing would change
                // the string used as the auth-cache key and sent in outgoing requests.
                if url.chars().any(char::is_control) {
                    return Err(Diagnostic::Usage(format!(
                        "refusing server value \"{}\": it contains a control character",
                        sanitize_for_diagnostic(url)
                    )));
                }

                tracing::debug!(server = url, "resolved server");

                validate_scheme(url, allow_insecure)?;

                Ok(Config { server: url.to_string() })
            }
        }
    }
}

/// Refuses a non-local `http://` server unless overridden.
///
/// Only the `http` scheme is checked: `https://` is always accepted, and a
/// value that is not a parseable absolute URL (e.g. an unrecognised bare
/// word — see `resolve_with_unknown_word_passes_through`) is left for a
/// later layer to reject on its own terms, since there is no scheme to
/// assess a cleartext risk on.
fn validate_scheme(server: &str, allow_insecure: bool) -> Result<(), Diagnostic> {
    if allow_insecure {
        return Ok(());
    }

    let Ok(parsed) = reqwest::Url::parse(server) else {
        return Ok(());
    };

    if parsed.scheme() == "http" && !is_local_host(&parsed) {
        return Err(Diagnostic::Usage(format!(
            "refusing to use \"{}\" over plain HTTP: an authenticated command sends a bearer \
token, which would cross the network in cleartext. Use https://, a local address \
(loopback or unspecified (127.0.0.0/8, ::1, 0.0.0.0, ::) or localhost), or override \
with --allow-insecure-server / DSP_ALLOW_INSECURE_SERVER=1.",
            sanitize_for_diagnostic(server)
        )));
    }

    Ok(())
}

/// Whether `url`'s host is loopback, unspecified (`0.0.0.0`/`::`), or
/// `localhost` — the set of hosts a bearer token never actually leaves the
/// machine for, even over plain HTTP.
///
/// `reqwest::Url` re-exports `url::Url`, so `.host()` is reachable — but not
/// the `url::Host` enum it returns, since `url` is not a direct dependency of
/// this crate to match on. This reads `host_str()` instead. An IPv6 host
/// comes back bracketed (e.g. `"[::1]"` — `url`'s `Host::Ipv6` `Display` impl
/// wraps in brackets); strip them before parsing with `std::net::IpAddr`. A
/// bare IPv4 host or domain has no brackets, so the strip is a no-op for
/// those.
fn is_local_host(url: &reqwest::Url) -> bool {
    let Some(host) = url.host_str() else { return false };

    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }

    let bare = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(host);
    bare.parse::<std::net::IpAddr>()
        .map(|ip| ip.is_loopback() || ip.is_unspecified())
        .unwrap_or(false)
}

/// Strips control characters (including ANSI escape, `\x1b`) from a
/// `--server` value before it is embedded in a diagnostic message, so a
/// crafted server value cannot inject terminal escapes into an error message.
fn sanitize_for_diagnostic(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Diagnostic;

    #[test]
    fn resolve_with_literal_url() {
        let cfg = Config::resolve(Some("https://api.example.org"), false).unwrap();
        assert_eq!(cfg.server, "https://api.example.org");
    }

    #[test]
    fn resolve_with_known_shortcut_prod() {
        let cfg = Config::resolve(Some("prod"), false).unwrap();
        assert_eq!(cfg.server, "https://api.dasch.swiss");
    }

    #[test]
    fn resolve_with_known_shortcut_local() {
        // "local" expands to http://0.0.0.0:3333 — an unspecified address, not
        // loopback — and must still resolve with no override (CRITICAL case:
        // this is the developer's primary shortcut, dsp-cli/ADR-0007).
        let cfg = Config::resolve(Some("local"), false).unwrap();
        assert_eq!(cfg.server, "http://0.0.0.0:3333");
    }

    #[test]
    fn resolve_with_unknown_word_passes_through() {
        let cfg = Config::resolve(Some("staging-experiment"), false).unwrap();
        assert_eq!(cfg.server, "staging-experiment");
    }

    #[test]
    fn resolve_with_known_shortcut_dev() {
        let cfg = Config::resolve(Some("dev"), false).unwrap();
        assert_eq!(cfg.server, "https://api.dev.dasch.swiss");
    }

    #[test]
    fn resolve_with_known_shortcut_demo() {
        let cfg = Config::resolve(Some("demo"), false).unwrap();
        assert_eq!(cfg.server, "https://api.demo.dasch.swiss");
    }

    #[test]
    fn resolve_shortcut_is_case_insensitive() {
        // Mixed and upper case both resolve to the canonical URL.
        assert_eq!(Config::resolve(Some("PROD"), false).unwrap().server, "https://api.dasch.swiss");
        assert_eq!(
            Config::resolve(Some("Dev"), false).unwrap().server,
            "https://api.dev.dasch.swiss"
        );
    }

    #[test]
    fn resolve_literal_url_preserves_case() {
        // A non-shortcut value passes through unchanged — casing is NOT lowered.
        // (Also proves the scheme-validation pass doesn't normalise the stored
        // value: it only *parses* a copy of `server` to inspect the scheme.)
        let cfg = Config::resolve(Some("https://API.Example.ORG/Path"), false).unwrap();
        assert_eq!(cfg.server, "https://API.Example.ORG/Path");
    }

    #[test]
    fn resolve_with_none_returns_usage_diagnostic() {
        let err = Config::resolve(None, false).unwrap_err();
        assert!(matches!(err, Diagnostic::Usage(_)));
    }

    #[test]
    fn missing_server_message_mentions_all_three_paths() {
        let err = Config::resolve(None, false).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("--server"), "missing --server in: {msg}");
        assert!(msg.contains("DSP_SERVER"), "missing DSP_SERVER in: {msg}");
        assert!(msg.contains(".env"), "missing .env in: {msg}");
    }

    #[test]
    fn shortcut_and_canonical_url_resolve_identically() {
        // set_entry(server, …) and token(server) both use the resolved URL as the map
        // key, so "dev" and its canonical expansion must produce the same string.
        let via_shortcut = Config::resolve(Some("dev"), false).unwrap();
        let via_url = Config::resolve(Some("https://api.dev.dasch.swiss"), false).unwrap();
        assert_eq!(
            via_shortcut.server, via_url.server,
            "shortcut 'dev' and its URL must resolve to the same string for \
cache key lookups to work"
        );
    }

    // ── --server scheme validation ────────────────────────────────────────────

    #[test]
    fn https_is_always_accepted() {
        assert!(Config::resolve(Some("https://api.dasch.swiss"), false).is_ok());
    }

    #[test]
    fn every_shortcut_still_resolves_with_scheme_validation() {
        // SHORTCUTS holds only https:// entries except "local" (0.0.0.0, covered
        // by resolve_with_known_shortcut_local above) — assert none of them are
        // rejected now that resolve() validates scheme.
        for (name, _) in SHORTCUTS {
            let result = Config::resolve(Some(name), false);
            assert!(result.is_ok(), "shortcut '{name}' must still resolve, got {result:?}");
        }
    }

    #[test]
    fn http_loopback_ipv6_with_brackets_is_accepted() {
        // Discriminator for the host_str()-returns-bracketed-IPv6 question.
        let cfg = Config::resolve(Some("http://[::1]:3333"), false).unwrap();
        assert_eq!(cfg.server, "http://[::1]:3333");
    }

    #[test]
    fn http_unspecified_ipv4_is_accepted() {
        let cfg = Config::resolve(Some("http://0.0.0.0:3333"), false).unwrap();
        assert_eq!(cfg.server, "http://0.0.0.0:3333");
    }

    #[test]
    fn http_loopback_ipv4_is_accepted() {
        let cfg = Config::resolve(Some("http://127.0.0.1:3333"), false).unwrap();
        assert_eq!(cfg.server, "http://127.0.0.1:3333");
    }

    #[test]
    fn http_localhost_is_accepted() {
        let cfg = Config::resolve(Some("http://localhost:3333"), false).unwrap();
        assert_eq!(cfg.server, "http://localhost:3333");
    }

    #[test]
    fn http_non_local_host_is_refused() {
        let err = Config::resolve(Some("http://api.example.org"), false).unwrap_err();
        assert!(matches!(err, Diagnostic::Usage(_)), "expected Usage, got {err:?}");
        let msg = err.to_string();
        assert!(msg.contains("cleartext") || msg.contains("bearer token"), "message: {msg}");
        assert!(msg.contains("--allow-insecure-server"), "message: {msg}");
        assert!(msg.contains("DSP_ALLOW_INSECURE_SERVER"), "message: {msg}");
    }

    #[test]
    fn http_non_local_host_passes_with_override() {
        // Represents both override paths: the CLI collapses --allow-insecure-server
        // and DSP_ALLOW_INSECURE_SERVER=1 into this single bool before calling
        // Config::resolve (see Cli::allow_insecure_server in src/cli/mod.rs); the
        // flag/env parsing itself is covered by CLI-layer tests, not here.
        let cfg = Config::resolve(Some("http://api.example.org"), true).unwrap();
        assert_eq!(cfg.server, "http://api.example.org");
    }

    #[test]
    fn control_character_in_server_value_is_refused() {
        // A control character (ESC here) makes a server value outright invalid,
        // even over https:// — refused before the scheme is ever checked. The
        // diagnostic text itself must not carry the raw byte back out.
        let server = "https://api.example.org/\u{1b}[31mFAKE\u{1b}[0m";
        let err = Config::resolve(Some(server), false).unwrap_err();
        assert!(matches!(err, Diagnostic::Usage(_)), "expected Usage, got {err:?}");
        let msg = err.to_string();
        assert!(
            msg.bytes().all(|b| b >= 0x20 || b == b'\n'),
            "control character leaked into diagnostic: {msg:?}"
        );
    }
}
