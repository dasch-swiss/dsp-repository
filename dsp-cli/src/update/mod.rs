//! Interactive update check (advise-only). See ADR-0015
//! (`docs/adr/0015-update-check-and-self-update.md`) and the 031 plan
//! (`docs/design/plans/031-update-check/implementation-plan.md`).
//!
//! On **prose-format + interactive-TTY** runs only (unless opted out via
//! `DSP_NO_UPDATE_CHECK`), `dsp` checks the crates.io sparse index for a
//! newer published version and, if one exists, prints a two-line advisory to
//! stderr recommending `cargo install dsp-cli`. No binary self-replace, no
//! shell-out to `cargo`, no change to stdout, the JSON envelope, or the exit
//! code — every failure path is non-fatal and swallowed.
//!
//! [`maybe_notify`] is the single public entry point; `main.rs` calls it
//! after handling the command result.

pub mod cache;

use std::io::Read;
use std::time::Duration;

use serde::Deserialize;

/// The crates.io sparse-index path for `dsp-cli`. Fixed by the crate name; if
/// the name ever changes, this path changes with it (see the 031 plan's
/// "Sparse-index path stability" risk note).
///
/// `fetch_latest` takes its URL as an explicit parameter (rather than reading
/// this const internally) so tests can point it at a mock server;
/// `run_check_and_notify` is the real caller that passes this const in
/// production.
pub(crate) const SPARSE_INDEX_URL: &str = "https://index.crates.io/ds/p-/dsp-cli";

/// Timeout for the update-check HTTP request. Kept short (~2s) since this
/// fetch runs synchronously on every gated interactive run and must never
/// noticeably delay the CLI.
const HTTP_TIMEOUT: Duration = Duration::from_secs(2);

/// At most one network fetch per this many hours; within the window the
/// reminder still shows every interactive run, from the cached
/// `latest_seen` (see ADR-0015 "Transport, frequency, politeness").
pub(crate) const CHECK_INTERVAL_HOURS: i64 = 24;

/// Standalone opt-out env var (set to any non-empty value to disable the
/// check entirely). Read directly via `std::env`, not through
/// `Config::resolve` (server-only) — see ADR-0015 "Opt-out".
pub(crate) const OPT_OUT_ENV: &str = "DSP_NO_UPDATE_CHECK";

/// Size cap on the response body read, for parity with `AuthCache`'s
/// anti-slurp guard. A crates.io sparse-index entry is a few KiB in practice;
/// this cap only protects against a misbehaving/malicious endpoint forcing
/// unbounded allocation.
const MAX_BODY_BYTES: u64 = 1 << 20;

/// One line of the crates.io sparse index (newline-delimited JSON, one
/// object per published version). Unknown fields (`cksum`, `deps`,
/// `features`, …) are silently ignored — we only need `vers` and `yanked`.
#[derive(Debug, Deserialize)]
struct IndexEntry {
    vers: String,
    #[serde(default)]
    yanked: bool,
}

/// Parses a crates.io sparse-index response body and returns the highest
/// published **stable** (non-prerelease, non-yanked) version, if any.
///
/// The index's `vers` field has no `v` prefix (e.g. `"0.1.3"`, not
/// `"v0.1.3"`); we do not strip one — see the test below.
///
/// Malformed lines, entries with an unparseable `vers`, yanked entries, and
/// prerelease versions are all silently skipped rather than treated as
/// errors — an empty/garbage body simply yields `None`.
pub fn parse_latest_stable(body: &str) -> Option<semver::Version> {
    body.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<IndexEntry>(line).ok())
        .filter(|entry| !entry.yanked)
        .filter_map(|entry| semver::Version::parse(&entry.vers).ok())
        .filter(|version| version.pre.is_empty())
        .max()
}

/// Fetches `url` (a crates.io sparse-index endpoint) and returns the highest
/// published stable version found in the response body, if any.
///
/// **Errors here are never user-visible; the caller logs at debug and drops
/// them.** Every failure path (client construction, the request itself, or
/// reading the body) maps to `Diagnostic::Internal` — never any other
/// `Diagnostic` variant — so a future refactor doesn't leak an `Internal`
/// diagnostic onto the `main.rs` `Error: {diag}` path. A non-2xx response
/// (404 for an unpublished crate name, 5xx for a server error) is not
/// treated as an error at all: it simply means "no info available", so it
/// returns `Ok(None)`.
///
/// `url` is an explicit parameter (rather than derived from `SPARSE_INDEX_URL`
/// internally) so tests can point this at a mock server.
pub fn fetch_latest(url: &str) -> Result<Option<semver::Version>, crate::diagnostic::Diagnostic> {
    let client = reqwest::blocking::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(crate::util::USER_AGENT)
        .build()
        .map_err(|e| {
            crate::diagnostic::Diagnostic::Internal(format!("failed to build update-check HTTP client: {e}"))
        })?;

    let response = client
        .get(url)
        .send()
        .map_err(|e| crate::diagnostic::Diagnostic::Internal(format!("update-check request failed: {e}")))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let mut buf = String::new();
    response.take(MAX_BODY_BYTES).read_to_string(&mut buf).map_err(|e| {
        crate::diagnostic::Diagnostic::Internal(format!("failed to read update-check response body: {e}"))
    })?;

    Ok(parse_latest_stable(&buf))
}

/// Pure gate predicate — no TTY/env access, so it is directly unit-testable.
/// `maybe_notify` reads the real TTY/env state and passes plain `bool`s here.
///
/// Open iff the effective format is `Prose`, stderr is an interactive TTY,
/// and the opt-out env var is not set (ADR-0015 "Where it runs and what
/// gates it").
fn gate_open(fmt: Option<crate::render::Format>, stderr_is_tty: bool, opted_out: bool) -> bool {
    matches!(fmt, Some(crate::render::Format::Prose)) && stderr_is_tty && !opted_out
}

/// Pure staleness check: `true` iff the cache has never been checked, or the
/// last check was at least `interval_hours` ago (boundary inclusive — exactly
/// `interval_hours` ago counts as stale).
fn is_stale(
    now: chrono::DateTime<chrono::Utc>,
    last_checked: Option<chrono::DateTime<chrono::Utc>>,
    interval_hours: i64,
) -> bool {
    match last_checked {
        None => true,
        Some(last) => now - last >= chrono::Duration::hours(interval_hours),
    }
}

/// Composes the two-line advisory printed to stderr. Plain language, no
/// "crate" (ADR-0001 vocabulary — dsp-cli's user-facing surface avoids the
/// word).
fn compose_notice(current: &semver::Version, latest: &semver::Version) -> String {
    format!(
        "A newer version of dsp is available: {latest} (you have {current}).\n\
         Upgrade with: cargo install dsp-cli   (or, if you use cargo-update: cargo install-update -a)"
    )
}

/// The single entry point: gates, then best-effort checks + notifies.
/// Swallows all errors; never alters the process exit code.
///
/// The gate (format + TTY + opt-out) is evaluated before any I/O, so the
/// common non-interactive/agent path costs nothing (ADR-0015).
pub fn maybe_notify(fmt: Option<crate::render::Format>) {
    use std::io::IsTerminal;

    let stderr_is_tty = std::io::stderr().is_terminal();
    let opted_out = std::env::var(OPT_OUT_ENV).map(|v| !v.is_empty()).unwrap_or(false);

    if !gate_open(fmt, stderr_is_tty, opted_out) {
        return;
    }

    if let Err(e) = run_check_and_notify() {
        tracing::debug!(error = %e, "update check failed; ignoring");
    }
}

/// Re-parses a cached `latest_seen` string through `semver::Version`.
///
/// This is the **sanitisation guard** on the cache→notice path: `latest_seen`
/// is a plain `String` round-tripped through TOML, so a corrupt file or a
/// future cache-format bug could hand back arbitrary text. Re-validating it
/// as a real `Version` here means garbage/control-char content silently
/// becomes `None` instead of ever reaching [`compose_notice`] and being
/// printed verbatim to a TTY (security review). Used for both outcome-matrix
/// branches in [`run_check_and_notify`] where the candidate falls back to the
/// cache instead of a fresh fetch.
fn resolve_cached_latest(latest_seen: &Option<String>) -> Option<semver::Version> {
    latest_seen.as_deref().and_then(|s| semver::Version::parse(s).ok())
}

/// Orchestrates one gated invocation: resolve the current version, load the
/// cache, fetch a fresh version if the cache is stale, and print the
/// advisory if a newer stable version is known. Never propagates a fetch or
/// cache-save error upward — see the outcome matrix in the 031 plan's Step 5.
fn run_check_and_notify() -> Result<(), crate::diagnostic::Diagnostic> {
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|e| crate::diagnostic::Diagnostic::Internal(format!("could not parse own version: {e}")))?;

    let mut cache = cache::UpdateCheckCache::load();
    let now = chrono::Utc::now();

    let latest: Option<semver::Version> = if is_stale(now, cache.last_checked, CHECK_INTERVAL_HOURS) {
        // Stamp before the fetch so the 24h backoff holds even on failure —
        // a failed attempt still counts as an attempt (ADR-0015).
        cache.last_checked = Some(now);

        let candidate = match fetch_latest(SPARSE_INDEX_URL) {
            Ok(Some(v)) => {
                cache.latest_seen = Some(v.to_string());
                Some(v)
            }
            Ok(None) => resolve_cached_latest(&cache.latest_seen),
            Err(e) => {
                // Documented non-fatal contract (ADR-0015): log at debug and
                // fall back to the last-known cached version, if any.
                tracing::debug!(error = %e, "update check fetch failed; using cached version if any");
                resolve_cached_latest(&cache.latest_seen)
            }
        };

        if let Err(e) = cache.save() {
            tracing::debug!(error = %e, "failed to save update check cache; ignoring");
        }

        candidate
    } else {
        resolve_cached_latest(&cache.latest_seen)
    };

    if let Some(latest) = latest
        && latest > current
    {
        eprintln!("{}", compose_notice(&current, &latest));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_version_body_returns_highest_stable() {
        let body = r#"{"name":"dsp-cli","vers":"0.1.0","yanked":false,"cksum":"abc"}
{"name":"dsp-cli","vers":"0.1.1","yanked":false,"cksum":"def"}
{"name":"dsp-cli","vers":"0.1.2","yanked":false,"cksum":"ghi"}
"#;
        assert_eq!(parse_latest_stable(body), Some(semver::Version::parse("0.1.2").unwrap()));
    }

    #[test]
    fn yanked_highest_entry_is_skipped() {
        let body = r#"{"name":"dsp-cli","vers":"0.1.1","yanked":false}
{"name":"dsp-cli","vers":"0.1.2","yanked":true}
"#;
        assert_eq!(parse_latest_stable(body), Some(semver::Version::parse("0.1.1").unwrap()));
    }

    #[test]
    fn prerelease_higher_than_latest_stable_is_skipped() {
        let body = r#"{"name":"dsp-cli","vers":"0.1.3","yanked":false}
{"name":"dsp-cli","vers":"0.2.0-rc1","yanked":false}
"#;
        assert_eq!(parse_latest_stable(body), Some(semver::Version::parse("0.1.3").unwrap()));
    }

    #[test]
    fn interleaved_malformed_lines_are_ignored() {
        let body = "not json\n\
{\"name\":\"dsp-cli\",\"vers\":\"0.1.0\",\"yanked\":false}\n\
\n\
{\"name\":\"dsp-cli\",\"vers\":\"0.1.1\",\"yanked\":false}\n";
        assert_eq!(parse_latest_stable(body), Some(semver::Version::parse("0.1.1").unwrap()));
    }

    #[test]
    fn empty_body_returns_none() {
        assert_eq!(parse_latest_stable(""), None);
    }

    #[test]
    fn all_malformed_or_all_yanked_returns_none() {
        let all_malformed = "not json\nalso not json\n";
        assert_eq!(parse_latest_stable(all_malformed), None);

        let all_yanked = r#"{"name":"dsp-cli","vers":"0.1.0","yanked":true}
{"name":"dsp-cli","vers":"0.1.1","yanked":true}
"#;
        assert_eq!(parse_latest_stable(all_yanked), None);
    }

    #[test]
    fn unparseable_vers_is_skipped_not_an_error() {
        let body = r#"{"vers":"not-a-version","yanked":false}
{"name":"dsp-cli","vers":"0.1.0","yanked":false}
"#;
        assert_eq!(parse_latest_stable(body), Some(semver::Version::parse("0.1.0").unwrap()));

        let only_unparseable = r#"{"vers":"not-a-version","yanked":false}"#;
        assert_eq!(parse_latest_stable(only_unparseable), None);
    }

    #[test]
    fn plain_version_without_v_prefix_parses_and_is_returned() {
        let body = r#"{"name":"dsp-cli","vers":"0.1.3","yanked":false,"cksum":"abc"}"#;
        assert_eq!(parse_latest_stable(body), Some(semver::Version::parse("0.1.3").unwrap()));
    }

    #[test]
    fn is_stale_when_never_checked() {
        let now = chrono::Utc::now();
        assert!(is_stale(now, None, CHECK_INTERVAL_HOURS));
    }

    #[test]
    fn is_stale_when_last_checked_25_hours_ago() {
        let now = chrono::Utc::now();
        let last = now - chrono::Duration::hours(25);
        assert!(is_stale(now, Some(last), CHECK_INTERVAL_HOURS));
    }

    #[test]
    fn is_fresh_when_last_checked_1_hour_ago() {
        let now = chrono::Utc::now();
        let last = now - chrono::Duration::hours(1);
        assert!(!is_stale(now, Some(last), CHECK_INTERVAL_HOURS));
    }

    #[test]
    fn is_stale_at_exact_24_hour_boundary() {
        let now = chrono::Utc::now();
        let last = now - chrono::Duration::hours(24);
        assert!(is_stale(now, Some(last), CHECK_INTERVAL_HOURS));
    }

    #[test]
    fn gate_open_when_prose_tty_and_not_opted_out() {
        assert!(gate_open(Some(crate::render::Format::Prose), true, false));
    }

    #[test]
    fn gate_closed_when_format_is_not_prose() {
        assert!(!gate_open(Some(crate::render::Format::Json), true, false));
    }

    #[test]
    fn gate_closed_when_format_is_none() {
        assert!(!gate_open(None, true, false));
    }

    #[test]
    fn gate_closed_when_opted_out() {
        assert!(!gate_open(Some(crate::render::Format::Prose), true, true));
    }

    #[test]
    fn gate_closed_when_stderr_is_not_a_tty() {
        assert!(!gate_open(Some(crate::render::Format::Prose), false, false));
    }

    #[test]
    fn compose_notice_contains_versions_and_install_command() {
        let current = semver::Version::parse("0.1.2").unwrap();
        let latest = semver::Version::parse("0.1.3").unwrap();
        let notice = compose_notice(&current, &latest);

        assert!(notice.contains("0.1.3"));
        assert!(notice.contains("0.1.2"));
        assert!(notice.contains("cargo install dsp-cli"));
    }

    #[test]
    fn compose_notice_is_exactly_two_lines() {
        let current = semver::Version::parse("0.1.2").unwrap();
        let latest = semver::Version::parse("0.1.3").unwrap();
        let notice = compose_notice(&current, &latest);

        assert_eq!(notice.lines().count(), 2);
    }

    #[test]
    fn compose_notice_never_says_crate() {
        let current = semver::Version::parse("0.1.2").unwrap();
        let latest = semver::Version::parse("0.1.3").unwrap();
        let notice = compose_notice(&current, &latest);

        assert!(!notice.to_lowercase().contains("crate"));
    }

    #[test]
    fn garbage_latest_seen_reparses_to_none_sanitisation_guard() {
        // Pins the "network-supplied/cached version is never printed
        // verbatim" invariant by calling the ACTUAL production function
        // `run_check_and_notify` uses on both outcome-matrix branches — not a
        // re-derivation of the same expression — so a regression in
        // `resolve_cached_latest` (e.g. swapping in an `unwrap_or_default`)
        // would fail this test.
        let latest_seen = Some("\u{7}not-a-version".to_string());
        assert_eq!(resolve_cached_latest(&latest_seen), None);
    }

    #[test]
    fn valid_cached_latest_seen_reparses_to_some_version() {
        // Companion happy-path case: a well-formed cached string round-trips
        // through `resolve_cached_latest` back to the same `Version`.
        let latest_seen = Some("0.1.5".to_string());
        assert_eq!(
            resolve_cached_latest(&latest_seen),
            Some(semver::Version::parse("0.1.5").unwrap())
        );
    }

    #[test]
    fn absent_cached_latest_seen_reparses_to_none() {
        assert_eq!(resolve_cached_latest(&None), None);
    }
}
