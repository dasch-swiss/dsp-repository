// Live integration test for `DspClient::resource_counts` (backs the
// `resource-type list --count` / `resource-type describe --count` flags).
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
// Token supply (all optional — `resource_counts` uses a public endpoint):
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
/// find the `beol` data-model, call `describe_resource_type` for `page` to
/// obtain its full class IRI (this is the "known-good" target this repo's
/// other orchestrated live tests already anchor on — see
/// `live_resource_type_describe.rs`, `live_resource_type_list.rs`,
/// `live_data_model_describe.rs`, `live_data_model_structure.rs`,
/// `live_data_model_list.rs` — all of which resolve project `0801`/beol via
/// `resolve_project` + `list_data_models`; 0803/incunabula is used elsewhere
/// in this repo only by tests that take a project/class-IRI directly via env
/// vars, a different pattern from this test's flow), then call
/// `resource_counts` and assert:
///
/// - Result is `Ok`.
/// - The returned map is non-empty (sanity check that the endpoint returned real data, not an
///   empty/degenerate response).
/// - **Risk 1 (class-IRI form match)**: the map contains the EXACT IRI returned by
///   `describe_resource_type` for `beol:page` as a key. This is the check that would fail if the v3
///   `resourcesPerOntology` route used a different class-IRI form than `allentities` (e.g.
///   different case, a different ontology version segment). `beol:page` is a project-defined
///   (non-builtin) class, so this is a FIRM assertion, not a soft check.
///
/// Note: the per-class count is typed `u64`, so non-negativity is a
/// type-level guarantee and is not asserted separately; the presence
/// assertion above is the real test of correctness.
///
/// Skips cleanly (with an `eprintln!`) if the required `DSP_TEST_SERVER`
/// environment variable is absent. Never fails due to missing config — only
/// due to real errors.
#[test]
#[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]
fn live_resource_counts_contains_known_class_iri() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim())).expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 2. Resolve optional token ─────────────────────────────────────────────
    // `resource_counts` uses a public endpoint; the token is optional. If
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

    // ── 5. List data-models and find the beol data-model ──────────────────────
    let data_models = client
        .list_data_models(&cfg.server, &proj.iri, token_ref)
        .expect("list_data_models failed — check DSP_TEST_SERVER and network connectivity");

    let beol_dm = data_models.iter().find(|dm| dm.name.eq_ignore_ascii_case("beol")).expect(
        "beol data-model not found in project '0801' — expected the beol data-model to be \
             present; check that DSP_TEST_SERVER points to a server where this project is \
             populated with its standard data-models",
    );

    eprintln!("live test: found beol data-model (iri: {})", beol_dm.iri);

    // ── 6. Describe beol:page to get its full class IRI (Risk 1 anchor) ──────
    let resource_type_name = "page";
    let detail = client
        .describe_resource_type(&cfg.server, &beol_dm.iri, resource_type_name, token_ref)
        .expect(
            "describe_resource_type failed for 'page' in beol — check DSP_TEST_SERVER and \
             network connectivity; if beol:page has been removed, update the test to use a \
             different well-known resource-type in this project",
        );

    eprintln!(
        "live test: describe_resource_type returned class IRI '{}' for 'beol:page'",
        detail.iri
    );

    // ── 7. Call resource_counts ────────────────────────────────────────────────
    eprintln!("live test: calling resource_counts for project IRI {}", proj.iri);

    let counts = client.resource_counts(&cfg.server, &proj.iri, token_ref).expect(
        "resource_counts failed — check DSP_TEST_SERVER and network connectivity; the \
             v3 resourcesPerOntology route requires dsp-api >= 37.1.0",
    );

    eprintln!(
        "live test: resource_counts returned {} class(es) across the project",
        counts.len()
    );

    // ── 8. Assert structural invariants ───────────────────────────────────────

    // Sanity check: the endpoint must return real data, not an empty map.
    assert!(
        !counts.is_empty(),
        "expected a non-empty resource-counts map for project '0801' but got an empty map. \
         Check that DSP_TEST_SERVER points to a server where the beol project has instance data."
    );

    // Risk 1: the v3 class-IRI form must match the allentities class-IRI form.
    // beol:page is a project-defined (non-builtin) class, so it MUST be present
    // in the v3 payload — this is a firm assertion, not a soft check.
    assert!(
        counts.contains_key(&detail.iri),
        "Risk 1 (plan 030): the resource-counts map does not contain the class IRI '{}' \
         returned by describe_resource_type for 'beol:page'. This indicates the v3 \
         resourcesPerOntology route uses a DIFFERENT class-IRI form than the allentities \
         endpoint (e.g. different case or ontology-version segment) — the merge keys used \
         by `resource-type list --count` / `describe --count` would silently fail to match. \
         Map keys observed: {:?}",
        detail.iri,
        counts.keys().collect::<Vec<_>>()
    );

    eprintln!(
        "live test: PASSED — resource_counts contains the known class IRI '{}' \
         (count={}), {} class(es) total",
        detail.iri,
        counts.get(&detail.iri).unwrap(),
        counts.len()
    );
}
