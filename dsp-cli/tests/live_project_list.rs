// Live integration test for `dsp vre project list`.
//
// This entire file is compiled and run ONLY when the `live` feature is active:
//   cargo test --features live --test 'live*'
//
// The `just test-live` recipe runs exactly this command.
//
// dsp-cli/ADR-0009 (testing strategy): live tests are **not** in CI. They require
// real environment variables pointing at a live DSP instance. Missing config
// causes an early-return skip — never a test failure.
//
// Required environment variables:
//   DSP_TEST_SERVER   — server URL or shortcut (e.g. "dev", "https://api.dev.dasch.swiss")
//
// Token supply (all optional — `list_projects` is a public endpoint):
//   DSP_TOKEN         — bearer token to send; if set, ensures the authenticated
//                       path is exercised. NEVER logged or interpolated in
//                       failure messages.
//
// NEVER log the token value regardless of which path supplies it.
#![cfg(feature = "live")]

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::config::Config;

mod common;
use common::{optional_env, require_env};

// ---------------------------------------------------------------------------
// Live test
// ---------------------------------------------------------------------------

/// End-to-end live test: call `GET /admin/projects` against a real server and
/// assert structural invariants on the returned `Vec<Project>`.
///
/// Skips cleanly (with an `eprintln!`) if the required `DSP_TEST_SERVER`
/// environment variable is absent. Never fails due to missing config — only
/// due to real errors.
///
/// Assertions are intentionally resilient to real-data variation:
/// - No hardcoded project counts or specific shortcodes.
/// - Shortcode shape: 4 hex digits — `len == 4` and all chars are ASCII hex.
/// - `data_models` is a `usize` — no panic on any valid count (including 0).
#[test]
#[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]
fn live_project_list_returns_non_empty_vec_with_valid_shortcodes() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim()), false)
        .expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 2. Resolve optional token ─────────────────────────────────────────────
    // `list_projects` is a public endpoint; the token is optional. If present
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

    // ── 4. Call list_projects ─────────────────────────────────────────────────
    eprintln!("live test: calling list_projects on {}", cfg.server);

    let projects = client
        .list_projects(&cfg.server, token_ref)
        .expect("list_projects failed — check DSP_TEST_SERVER and network connectivity");

    eprintln!("live test: received {} projects", projects.len());

    // ── 5. Assert structural invariants ──────────────────────────────────────
    // A real DSP server always has at least one project (the system project
    // or any active research project). An empty list would indicate a server
    // misconfiguration or an unexpected API change.
    assert!(
        !projects.is_empty(),
        "expected at least one project from a live DSP server, but got an empty list. \
         Check that DSP_TEST_SERVER points to a populated server."
    );

    // Assert per-project structural invariants.
    for proj in &projects {
        // Shortcode must be exactly 4 ASCII hex digits (e.g. "0001", "ABCD").
        // This is the canonical DSP-API contract. Case: DSP-API returns lower-
        // case hex; we accept both upper and lower here (is_ascii_hexdigit covers
        // both), keeping the assertion resilient to server casing variations.
        assert_eq!(
            proj.shortcode.len(),
            4,
            "shortcode '{}' is not 4 characters (project iri: {})",
            proj.shortcode,
            proj.iri
        );
        assert!(
            proj.shortcode.chars().all(|c| c.is_ascii_hexdigit()),
            "shortcode '{}' contains non-hex characters (project iri: {})",
            proj.shortcode,
            proj.iri
        );

        // IRI must be non-empty and start with "http" (DSP-API IRIs are HTTP URIs).
        assert!(
            !proj.iri.is_empty(),
            "project with shortcode '{}' has an empty IRI",
            proj.shortcode
        );
        assert!(
            proj.iri.starts_with("http"),
            "project IRI '{}' does not start with 'http' (shortcode: {})",
            proj.iri,
            proj.shortcode
        );

        // Shortname must be non-empty.
        assert!(
            !proj.shortname.is_empty(),
            "project with shortcode '{}' has an empty shortname",
            proj.shortcode
        );

        // data_models is a usize — any value is structurally valid (no panic).
        // We only log it; no bound assertion here because valid projects may
        // have 0 or many data models.
        eprintln!(
            "live test: project {} ({}) — data_models {}",
            proj.shortcode, proj.shortname, proj.data_models
        );
    }

    eprintln!("live test: PASSED — all {} projects passed structural checks", projects.len());
}
