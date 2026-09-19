// Live integration test for `dsp vre project describe`.
//
// This entire file is compiled and run ONLY when the `live` feature is active:
//   cargo test --features live --test 'live*'
//
// The `just test-live` recipe runs exactly this command.
//
// ADR-0009 (testing strategy): live tests are **not** in CI. They require
// real environment variables pointing at a live DSP instance. Missing config
// causes an early-return skip — never a test failure.
//
// Required environment variables:
//   DSP_TEST_SERVER   — server URL or shortcut (e.g. "dev", "https://api.dev.dasch.swiss")
//   DSP_TEST_PROJECT  — shortcode, shortname, or IRI of a project to describe
//
// Token supply (all optional — `describe_project` is a public endpoint):
//   DSP_TOKEN         — bearer token to send; if set, ensures the authenticated
//                       path is exercised. NEVER logged or interpolated in
//                       failure messages.
//
// NEVER log the token value regardless of which path supplies it.
#![cfg(feature = "live")]

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::config::Config;

// ---------------------------------------------------------------------------
// Helpers (mirror live_project_list.rs exactly)
// ---------------------------------------------------------------------------

/// Read a required environment variable. Returns `None` and emits a skip
/// message if the variable is absent or empty.
fn require_env(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => {
            eprintln!("skipping live test: {name} not set");
            None
        }
    }
}

/// Read an optional environment variable. Returns `None` silently if absent.
fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

// ---------------------------------------------------------------------------
// Live test
// ---------------------------------------------------------------------------

/// End-to-end live test: call `GET /admin/projects/<identifier>` against a
/// real server and assert structural invariants on the returned `ProjectDetail`.
///
/// Skips cleanly (with an `eprintln!`) if any required environment variable
/// is absent. Never fails due to missing config — only due to real errors.
///
/// Assertions are intentionally resilient to real-data variation:
/// - Shortcode shape: 4 hex digits — `len == 4` and all chars are ASCII hex.
/// - Shortname must be non-empty.
/// - IRI must be non-empty and start with "http".
/// - `data_models` count is asserted `>= 0` (any valid value); individual
///   data-model names are asserted non-empty and IRIs start with "http".
/// - Status values: both `Active` and `Inactive` are acceptable.
#[test]
fn live_project_describe_returns_valid_project_detail() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };
    let project_input = match require_env("DSP_TEST_PROJECT") {
        Some(v) => v,
        None => return,
    };

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim()))
        .expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 2. Resolve optional token ─────────────────────────────────────────────
    // `describe_project` is a public endpoint; the token is optional. If present
    // it exercises the authenticated code path, but its absence is not a skip
    // condition. NEVER interpolate the token value in any log or assertion
    // message.
    let token: Option<String> = optional_env("DSP_TOKEN");
    let token_ref: Option<&str> = token.as_deref();

    if token_ref.is_some() {
        eprintln!("live test: DSP_TOKEN is set — exercising authenticated path");
    } else {
        eprintln!("live test: DSP_TOKEN not set — exercising anonymous path");
    }

    // ── 3. Build client ───────────────────────────────────────────────────────
    let client = HttpDspClient::new().expect("failed to build HTTP client");

    // ── 4. Call describe_project ──────────────────────────────────────────────
    eprintln!(
        "live test: calling describe_project on {} for project '{}'",
        cfg.server, project_input
    );

    let detail = client
        .describe_project(&cfg.server, project_input.trim(), token_ref)
        .expect(
            "describe_project failed — check DSP_TEST_SERVER, DSP_TEST_PROJECT, \
             and network connectivity",
        );

    eprintln!(
        "live test: received project detail for '{}' ({})",
        detail.shortname, detail.shortcode
    );

    // ── 5. Assert structural invariants ──────────────────────────────────────

    // Shortcode must be exactly 4 ASCII hex digits (e.g. "0001", "0801").
    assert_eq!(
        detail.shortcode.len(),
        4,
        "shortcode '{}' is not 4 characters",
        detail.shortcode
    );
    assert!(
        detail.shortcode.chars().all(|c| c.is_ascii_hexdigit()),
        "shortcode '{}' contains non-hex characters",
        detail.shortcode
    );

    // IRI must be non-empty and start with "http".
    assert!(
        !detail.iri.is_empty(),
        "project IRI must not be empty (shortcode: {})",
        detail.shortcode
    );
    assert!(
        detail.iri.starts_with("http"),
        "project IRI '{}' does not start with 'http' (shortcode: {})",
        detail.iri,
        detail.shortcode
    );

    // Shortname must be non-empty.
    assert!(
        !detail.shortname.is_empty(),
        "project shortname must not be empty (shortcode: {})",
        detail.shortcode
    );

    // When the input was a shortcode, assert the round-trip.
    // A 4-hex-digit input is unambiguously a shortcode — the same value
    // should be returned as the resolved shortcode.
    let trimmed = project_input.trim();
    let is_shortcode_input = trimmed.len() == 4 && trimmed.chars().all(|c| c.is_ascii_hexdigit());
    if is_shortcode_input {
        assert_eq!(
            detail.shortcode,
            trimmed.to_lowercase(),
            "describe_project by shortcode '{}' should return the same shortcode (got '{}')",
            trimmed,
            detail.shortcode
        );
    }

    // data_models: any count >= 0 is structurally valid. Assert per-entry invariants.
    eprintln!(
        "live test: {} data-model(s) — {:?}",
        detail.data_models.len(),
        detail
            .data_models
            .iter()
            .map(|dm| &dm.name)
            .collect::<Vec<_>>()
    );
    for dm in &detail.data_models {
        assert!(
            !dm.name.is_empty(),
            "data-model name must not be empty (project: {}, iri: {})",
            detail.shortcode,
            dm.iri
        );
        assert!(
            !dm.iri.is_empty(),
            "data-model IRI must not be empty (project: {}, name: {})",
            detail.shortcode,
            dm.name
        );
        assert!(
            dm.iri.starts_with("http"),
            "data-model IRI '{}' does not start with 'http' (project: {})",
            dm.iri,
            detail.shortcode
        );
    }

    eprintln!(
        "live test: project {} ({}) — status {:?}, {} data-model(s), \
         {} keyword(s), {} description(s)",
        detail.shortcode,
        detail.shortname,
        detail.status,
        detail.data_models.len(),
        detail.keywords.len(),
        detail.description.len()
    );

    eprintln!("live test: PASSED");
}
