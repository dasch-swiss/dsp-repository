// Live integration test for `dsp vre data-model list`.
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
// Token supply (all optional — `list_data_models` uses a public endpoint):
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

/// End-to-end live test: resolve a known project (beol / shortcode `0801`)
/// then call `list_data_models` against a real server and assert structural
/// invariants on the returned `Vec<DataModel>`.
///
/// Skips cleanly (with an `eprintln!`) if the required `DSP_TEST_SERVER`
/// environment variable is absent. Never fails due to missing config — only
/// due to real errors.
///
/// Assertions are intentionally resilient to real-data variation:
/// - Non-empty list (beol is an active research project with at least one project-defined
///   data-model).
/// - Every `name` is non-empty.
/// - Every returned item has `is_builtin == false` — the client method fetches project-scoped
///   data-models only; builtins are appended by the action, never by `list_data_models` itself.
/// - Every `iri` is non-empty and starts with `"http"`.
#[test]
#[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]
fn live_data_model_list_returns_non_empty_vec_with_valid_data_models() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim()), false)
        .expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 2. Resolve optional token ─────────────────────────────────────────────
    // `list_data_models` uses a public endpoint; the token is optional. If
    // present it exercises the authenticated code path, but its absence is not
    // a skip condition. NEVER interpolate the token value in any log or
    // assertion message.
    let token: Option<String> = optional_env("DSP_TOKEN");
    let token_ref: Option<&str> = token.as_deref();

    if token_ref.is_some() {
        eprintln!("live test: DSP_TOKEN is set — exercising authenticated path");
    } else {
        eprintln!("live test: DSP_TOKEN not set — exercising anonymous path");
    }

    // ── 3. Build client ───────────────────────────────────────────────────────
    let client = HttpDspClient::new().expect("failed to build HTTP client");

    // ── 4. Resolve the known project (beol / shortcode 0801) ─────────────────
    // Using a well-known active DaSCH research project that has at least one
    // project-defined data-model, so the assertion `!data_models.is_empty()` is
    // reliable. The shortcode `0801` is the canonical beol project.
    let project_identifier = "0801";
    eprintln!("live test: resolving project '{}' on {}", project_identifier, cfg.server);

    let proj = client.resolve_project(&cfg.server, project_identifier).expect(
        "resolve_project failed for '0801' — check DSP_TEST_SERVER and network connectivity; \
             if the beol project has been removed from this server, update the test to use a \
             different well-known project shortcode",
    );

    eprintln!(
        "live test: resolved project {} (shortcode {}, shortname {})",
        proj.iri, proj.shortcode, proj.shortname
    );

    // ── 5. Call list_data_models ──────────────────────────────────────────────
    eprintln!("live test: calling list_data_models for project IRI {}", proj.iri);

    let data_models = client
        .list_data_models(&cfg.server, &proj.iri, token_ref)
        .expect("list_data_models failed — check DSP_TEST_SERVER and network connectivity");

    eprintln!("live test: received {} data-model(s)", data_models.len());

    // ── 6. Assert structural invariants ──────────────────────────────────────

    // beol (0801) is an active research project that must have at least one
    // project-defined data-model. An empty list indicates a server issue or
    // an unexpected API change.
    assert!(
        !data_models.is_empty(),
        "expected at least one data-model for project '{}' ({}) but got an empty list. \
         Check that DSP_TEST_SERVER points to a server where this project is populated.",
        proj.shortcode,
        proj.shortname
    );

    // Assert per-data-model structural invariants.
    for dm in &data_models {
        // name must be non-empty (derived from the IRI at the client boundary).
        assert!(
            !dm.name.is_empty(),
            "data-model name must not be empty (project: {}, iri: {})",
            proj.shortcode,
            dm.iri
        );

        // IRI must be non-empty and start with "http".
        assert!(
            !dm.iri.is_empty(),
            "data-model IRI must not be empty (project: {}, name: {})",
            proj.shortcode,
            dm.name
        );
        assert!(
            dm.iri.starts_with("http"),
            "data-model IRI '{}' does not start with 'http' (project: {}, name: {})",
            dm.iri,
            proj.shortcode,
            dm.name
        );

        // `list_data_models` fetches project-scoped data-models only — builtins
        // are appended by the action when `--include-builtins` is set; they must
        // NOT appear in the raw client response.
        assert!(
            !dm.is_builtin,
            "data-model '{}' ({}) has is_builtin == true, but list_data_models must \
             never return built-ins — only project-defined data-models (project: {})",
            dm.name, dm.iri, proj.shortcode
        );

        eprintln!(
            "live test: data-model '{}' (iri: {}, label: {:?}, last_modified: {:?}, is_builtin: {})",
            dm.name, dm.iri, dm.label, dm.last_modified, dm.is_builtin
        );
    }

    eprintln!(
        "live test: PASSED — all {} data-model(s) for project '{}' ({}) passed structural checks",
        data_models.len(),
        proj.shortcode,
        proj.shortname
    );
}
