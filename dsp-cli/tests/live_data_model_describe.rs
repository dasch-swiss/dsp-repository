// Live integration test for `dsp vre data-model describe`.
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
// Token supply (all optional — `describe_data_model` uses a public endpoint):
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

/// End-to-end live test: resolve project `0801` (beol), list its data-models,
/// find the `beol` data-model, then call `describe_data_model` against a real
/// server and assert structural invariants on the returned `DataModelDetail`.
///
/// Skips cleanly (with an `eprintln!`) if the required `DSP_TEST_SERVER`
/// environment variable is absent. Never fails due to missing config — only
/// due to real errors.
///
/// Assertions are intentionally resilient to real-data variation:
/// - Data-model `name == "beol"` (we explicitly find and describe the beol data-model — a stable
///   fixture on the beol project).
/// - `label` is `Some` (the beol data-model has a label on the live API).
/// - `resource_types` is non-empty (beol defines multiple resource-types).
/// - Every resource-type `name` is non-empty.
/// - Every resource-type `iri` is non-empty and starts with `"http"`.
#[test]
#[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]
fn live_data_model_describe_returns_valid_data_model_detail() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim())).expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 2. Resolve optional token ─────────────────────────────────────────────
    // `describe_data_model` uses a public endpoint; the token is optional. If
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

    // ── 5. List data-models and find the beol data-model ─────────────────────
    eprintln!("live test: calling list_data_models for project IRI {}", proj.iri);

    let data_models = client
        .list_data_models(&cfg.server, &proj.iri, token_ref)
        .expect("list_data_models failed — check DSP_TEST_SERVER and network connectivity");

    eprintln!("live test: received {} data-model(s)", data_models.len());

    // Find the beol data-model by name (case-insensitive, mirroring the action).
    let beol_dm = data_models.iter().find(|dm| dm.name.eq_ignore_ascii_case("beol")).expect(
        "beol data-model not found in project '0801' — expected the beol data-model to be \
             present; check that DSP_TEST_SERVER points to a server where this project is \
             populated with its standard data-models",
    );

    eprintln!("live test: found beol data-model (iri: {})", beol_dm.iri);

    // ── 6. Call describe_data_model ───────────────────────────────────────────
    eprintln!("live test: calling describe_data_model for IRI {}", beol_dm.iri);

    let detail = client
        .describe_data_model(&cfg.server, &beol_dm.iri, token_ref)
        .expect("describe_data_model failed — check DSP_TEST_SERVER and network connectivity");

    eprintln!(
        "live test: received data-model detail for '{}' ({} resource-type(s))",
        detail.name,
        detail.resource_types.len()
    );

    // ── 7. Assert structural invariants ──────────────────────────────────────

    // name must be "beol" — we resolved this data-model by name.
    assert_eq!(detail.name, "beol", "data-model name must be 'beol'; got '{}'", detail.name);

    // label must be Some — the beol data-model has a label on the live API.
    assert!(
        detail.label.is_some(),
        "data-model 'beol' label must be Some on the live API; got None. \
         If the label was removed, update this assertion."
    );

    // resource_types must be non-empty — beol defines multiple resource-types.
    assert!(
        !detail.resource_types.is_empty(),
        "data-model 'beol' must have at least one resource-type; got an empty list. \
         Check that DSP_TEST_SERVER points to a server where the beol data-model is populated."
    );

    // Assert per-resource-type structural invariants.
    for rt in &detail.resource_types {
        assert!(
            !rt.name.is_empty(),
            "resource-type name must not be empty (data-model: {}, iri: {})",
            detail.name,
            rt.iri
        );
        assert!(
            !rt.iri.is_empty(),
            "resource-type IRI must not be empty (data-model: {}, name: {})",
            detail.name,
            rt.name
        );
        assert!(
            rt.iri.starts_with("http"),
            "resource-type IRI '{}' does not start with 'http' (data-model: {}, name: {})",
            rt.iri,
            detail.name,
            rt.name
        );

        eprintln!(
            "live test: resource-type '{}' (iri: {}, label: {:?})",
            rt.name, rt.iri, rt.label
        );
    }

    eprintln!(
        "live test: PASSED — data-model '{}' has {} resource-type(s), label={:?}",
        detail.name,
        detail.resource_types.len(),
        detail.label
    );
}
