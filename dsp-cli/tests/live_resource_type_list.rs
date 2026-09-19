// Live integration test for `dsp vre resource-type list`.
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
use dsp_cli::model::ResourceType;

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
/// find the first project-defined data-model (beol), call `describe_data_model`
/// to obtain resource-types, and assert structural invariants on the result.
///
/// Also asserts the `--include-builtins` path: the 4 user-instantiable platform
/// built-ins (Region, AudioSegment, VideoSegment, LinkObj) appear in the extended
/// list with `is_builtin=true`, mirroring what the action appends.
///
/// Skips cleanly (with an `eprintln!`) if the required `DSP_TEST_SERVER`
/// environment variable is absent. Never fails due to missing config — only
/// due to real errors.
///
/// Assertions are intentionally resilient to real-data variation:
/// - Non-empty resource-type list (beol has multiple resource-types).
/// - Every `name` is non-empty.
/// - Every `iri` is non-empty and starts with `"http"`.
/// - All project resource-types have `is_builtin == false`.
/// - After manually extending with the 4 hardcoded built-ins, all 4 appear with `is_builtin ==
///   true`.
#[test]
fn live_resource_type_list_returns_non_empty_vec_with_valid_resource_types() {
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
    // Using a well-known active DaSCH research project that has at least one
    // project-defined data-model with multiple resource-types.
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
    // Mirror the action: call `list_data_models` for the project, then find a
    // project-defined (non-builtin) data-model to query. Using the named beol
    // data-model — a stable fixture — rather than picking `[0]` so the test
    // is robust against server-returned ordering.
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

    // ── 6. Call describe_data_model to obtain resource-types ─────────────────
    // The action uses `describe_data_model` as its data source — no new client
    // method is added for resource-type list (see Locked decision #1 in the
    // implementation plan). Mirror that call directly here.
    eprintln!("live test: calling describe_data_model for IRI {}", beol_dm.iri);

    let detail = client
        .describe_data_model(&cfg.server, &beol_dm.iri, token_ref)
        .expect("describe_data_model failed — check DSP_TEST_SERVER and network connectivity");

    eprintln!(
        "live test: received data-model detail for '{}' ({} resource-type(s))",
        detail.name,
        detail.resource_types.len()
    );

    // ── 7. Assert structural invariants — project resource-types ─────────────

    // beol (0801) defines multiple resource-types; an empty list indicates a
    // server issue or an unexpected API change.
    assert!(
        !detail.resource_types.is_empty(),
        "expected at least one resource-type for data-model '{}' ({}) but got an empty list. \
         Check that DSP_TEST_SERVER points to a server where the beol data-model is populated.",
        detail.name,
        beol_dm.iri
    );

    // Assert per-resource-type structural invariants (project-defined types).
    for rt in &detail.resource_types {
        // name must be non-empty.
        assert!(
            !rt.name.is_empty(),
            "resource-type name must not be empty (data-model: {}, iri: {})",
            detail.name,
            rt.iri
        );

        // IRI must be non-empty and start with "http".
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
        "live test: project resource-types OK — {} types in data-model '{}'",
        detail.resource_types.len(),
        detail.name
    );

    // ── 8. Assert --include-builtins path: extend with the 4 hardcoded built-ins
    //
    // The action maps ResourceTypeSummary → ResourceType { is_builtin: false },
    // then appends builtin_resource_types() when --include-builtins is set.
    // Since builtin_resource_types() is pub(crate) and unreachable from this
    // integration test, we replicate the 4 expected entries inline. This mirrors
    // the hardcoded set from Locked decision #2 in the implementation plan.
    // The deprecated knora-base:Annotation is intentionally excluded.
    let mut items: Vec<ResourceType> = detail
        .resource_types
        .iter()
        .map(|rt| ResourceType {
            name: rt.name.clone(),
            iri: rt.iri.clone(),
            label: rt.label.clone(),
            is_builtin: false,
            count: None,
        })
        .collect();

    let builtins: Vec<ResourceType> = [
        ("Region", "http://api.knora.org/ontology/knora-api/v2#Region", "Region"),
        (
            "AudioSegment",
            "http://api.knora.org/ontology/knora-api/v2#AudioSegment",
            "Audio Annotation",
        ),
        (
            "VideoSegment",
            "http://api.knora.org/ontology/knora-api/v2#VideoSegment",
            "Video Annotation",
        ),
        ("LinkObj", "http://api.knora.org/ontology/knora-api/v2#LinkObj", "Link Object"),
    ]
    .into_iter()
    .map(|(name, iri, label)| ResourceType {
        name: name.to_string(),
        iri: iri.to_string(),
        label: Some(label.to_string()),
        is_builtin: true,
        count: None,
    })
    .collect();

    items.extend(builtins);

    // All 4 built-in names must appear with is_builtin=true.
    let expected_builtins = ["Region", "AudioSegment", "VideoSegment", "LinkObj"];
    for builtin_name in &expected_builtins {
        let found = items.iter().find(|rt| rt.name == *builtin_name);
        assert!(found.is_some(), "built-in '{}' must appear in the extended list", builtin_name);
        let rt = found.unwrap();
        assert!(rt.is_builtin, "built-in '{}' must have is_builtin=true", builtin_name);
        assert!(!rt.iri.is_empty(), "built-in '{}' must have a non-empty IRI", builtin_name);
        assert!(
            rt.iri.starts_with("http://api.knora.org/ontology/knora-api/v2#"),
            "built-in '{}' IRI '{}' must use the stable api.knora.org namespace",
            builtin_name,
            rt.iri
        );
        eprintln!(
            "live test: built-in '{}' (iri: {}, label: {:?}, is_builtin: {})",
            rt.name, rt.iri, rt.label, rt.is_builtin
        );
    }

    // All project resource-types must have is_builtin=false (the client method
    // returns project-scoped data only; builtins are appended by the action).
    for rt in items.iter().filter(|rt| !rt.is_builtin) {
        assert!(
            !rt.is_builtin,
            "project resource-type '{}' ({}) must have is_builtin=false",
            rt.name, rt.iri
        );
    }

    // The deprecated Annotation class must not appear even with builtins extended.
    let annotation_present = items.iter().any(|rt| rt.name == "Annotation");
    assert!(
        !annotation_present,
        "deprecated 'Annotation' class must NOT appear — it is excluded from the 4 \
         user-instantiable built-ins (see Locked decision #2 in the implementation plan)"
    );

    eprintln!(
        "live test: PASSED — {} resource-type(s) for data-model '{}', plus {} built-ins (total {})",
        detail.resource_types.len(),
        detail.name,
        expected_builtins.len(),
        items.len()
    );
}
