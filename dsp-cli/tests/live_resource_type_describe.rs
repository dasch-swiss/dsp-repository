// Live integration test for `dsp vre resource-type describe`.
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
//
// Token supply (all optional — `describe_resource_type` uses a public endpoint):
//   DSP_TOKEN         — bearer token to send; if set, ensures the authenticated
//                       path is exercised. NEVER logged or interpolated in
//                       failure messages.
//
// NEVER log the token value regardless of which path supplies it.
#![cfg(feature = "live")]

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::config::Config;
use dsp_cli::model::Representation;

// ---------------------------------------------------------------------------
// Helpers (mirror live_data_model_list.rs exactly)
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

/// End-to-end live test: resolve project `0801` (beol), list its data-models,
/// find the `beol` data-model, then call `describe_resource_type` for
/// `page` — a well-known still-image–bearing resource-type in beol — and
/// assert structural invariants on the returned `ResourceTypeDetail`.
///
/// Skips cleanly (with an `eprintln!`) if the required `DSP_TEST_SERVER`
/// environment variable is absent. Never fails due to missing config — only
/// due to real errors.
///
/// Assertions are intentionally resilient to real-data variation:
/// - Result is `Ok`.
/// - `fields` is non-empty (beol:page defines multiple project fields).
/// - `fields` contains a field named `seqnum` (the sequence-number property
///   on beol:page — a stable fixture on the live beol data-model).
/// - `representation` is `Some(Representation::StillImage)` (beol:page is a
///   still-image–bearing type via its flattened file-value restriction;
///   verified against the live beol API per Decision 5 / R8 of the plan).
#[test]
fn live_resource_type_describe_returns_valid_detail() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim()))
        .expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 2. Resolve optional token ─────────────────────────────────────────────
    // `describe_resource_type` uses a public endpoint; the token is optional. If
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
    // project-defined data-model with multiple resource-types, including a
    // well-known still-image–bearing type (`page`).
    let project_identifier = "0801";
    eprintln!(
        "live test: resolving project '{}' on {}",
        project_identifier, cfg.server
    );

    let proj = client
        .resolve_project(&cfg.server, project_identifier)
        .expect(
            "resolve_project failed for '0801' — check DSP_TEST_SERVER and network connectivity; \
             if the beol project has been removed from this server, update the test to use a \
             different well-known project shortcode",
        );

    eprintln!(
        "live test: resolved project {} (shortcode {}, shortname {})",
        proj.iri, proj.shortcode, proj.shortname
    );

    // ── 5. List data-models and find the beol data-model ─────────────────────
    // Mirror the action: call `list_data_models` for the project, then find the
    // beol data-model by name. Using the named beol data-model — a stable
    // fixture — rather than picking `[0]` so the test is robust against
    // server-returned ordering.
    eprintln!(
        "live test: calling list_data_models for project IRI {}",
        proj.iri
    );

    let data_models = client
        .list_data_models(&cfg.server, &proj.iri, token_ref)
        .expect("list_data_models failed — check DSP_TEST_SERVER and network connectivity");

    eprintln!("live test: received {} data-model(s)", data_models.len());

    // Find the beol data-model by name (case-insensitive, mirroring the action).
    let beol_dm = data_models
        .iter()
        .find(|dm| dm.name.eq_ignore_ascii_case("beol"))
        .expect(
            "beol data-model not found in project '0801' — expected the beol data-model to be \
             present; check that DSP_TEST_SERVER points to a server where this project is \
             populated with its standard data-models",
        );

    eprintln!("live test: found beol data-model (iri: {})", beol_dm.iri);

    // ── 6. Call describe_resource_type for beol:page ──────────────────────────
    // `page` is the well-known still-image–bearing resource-type in beol.
    // Its flattened `owl:Restriction`s include `knora-api:hasStillImageFileValue`
    // (verified on the live API per Decision 5 / R8 of the plan), so
    // `representation` must be `Some(Representation::StillImage)`.
    let resource_type_name = "page";
    eprintln!(
        "live test: calling describe_resource_type '{}' for IRI {}",
        resource_type_name, beol_dm.iri
    );

    let detail = client
        .describe_resource_type(&cfg.server, &beol_dm.iri, resource_type_name, token_ref)
        .expect(
            "describe_resource_type failed for 'page' in beol — check DSP_TEST_SERVER and \
             network connectivity; if beol:page has been removed, update the test to use a \
             different well-known still-image resource-type in this project",
        );

    eprintln!(
        "live test: received resource-type detail for '{}' ({} field(s), representation={:?})",
        detail.name,
        detail.fields.len(),
        detail.representation
    );

    // ── 7. Assert structural invariants ──────────────────────────────────────

    // name must be "page" — we resolved this resource-type by name.
    assert_eq!(
        detail.name, "page",
        "resource-type name must be 'page'; got '{}'",
        detail.name
    );

    // data_model must be "beol" — we described from the beol data-model.
    assert_eq!(
        detail.data_model, "beol",
        "resource-type data_model must be 'beol'; got '{}'",
        detail.data_model
    );

    // IRI must be non-empty and start with "http".
    assert!(
        !detail.iri.is_empty(),
        "resource-type IRI must not be empty"
    );
    assert!(
        detail.iri.starts_with("http"),
        "resource-type IRI '{}' must start with 'http'",
        detail.iri
    );

    // fields must be non-empty — beol:page defines multiple project fields.
    assert!(
        !detail.fields.is_empty(),
        "expected at least one field for 'beol:page' but got an empty list. \
         Check that DSP_TEST_SERVER points to a server where the beol data-model is populated."
    );

    eprintln!("live test: field count OK ({} fields)", detail.fields.len());

    // All fields must have non-empty names and IRIs.
    for field in &detail.fields {
        assert!(
            !field.name.is_empty(),
            "field name must not be empty (resource-type: {}, field iri: {})",
            detail.name,
            field.iri
        );
        assert!(
            !field.iri.is_empty(),
            "field IRI must not be empty (resource-type: {}, field name: {})",
            detail.name,
            field.name
        );
        eprintln!(
            "live test: field '{}' (iri: {}, value_type: {:?}, cardinality: {:?}, is_builtin: {})",
            field.name, field.iri, field.value_type, field.cardinality, field.is_builtin
        );
    }

    // fields must contain 'seqnum' — the sequence-number field is a stable
    // fixture of beol:page on the live API. Assert presence by name only
    // (not an exact match of all fields — live data may add fields over time).
    let has_seqnum = detail.fields.iter().any(|f| f.name == "seqnum");
    assert!(
        has_seqnum,
        "expected field 'seqnum' in 'beol:page' field list but it was not found. \
         Fields returned: {:?}. \
         If 'seqnum' has been renamed in the live beol data-model, update this assertion.",
        detail
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
    );

    eprintln!("live test: 'seqnum' field found — OK");

    // representation must be Some(StillImage) — beol:page is a still-image
    // representation (verified per Decision 5 / R8: representation is detected
    // from the flattened hasStillImageFileValue file-value restriction, not the
    // superclass ref).
    assert_eq!(
        detail.representation,
        Some(Representation::StillImage),
        "expected representation=Some(StillImage) for 'beol:page'; got {:?}. \
         If beol:page no longer bears a still-image file value, update this assertion.",
        detail.representation
    );

    eprintln!("live test: representation=StillImage — OK");

    eprintln!(
        "live test: PASSED — 'beol:page' has {} field(s), representation={:?}",
        detail.fields.len(),
        detail.representation
    );
}
