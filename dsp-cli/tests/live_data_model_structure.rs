// Live integration test for `dsp vre data-model structure`.
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
// Token supply (all optional — `data_model_structure` uses a public endpoint):
//   DSP_TOKEN         — bearer token to send; if set, ensures the authenticated
//                       path is exercised. NEVER logged or interpolated in
//                       failure messages.
//
// NEVER log the token value regardless of which path supplies it.
#![cfg(feature = "live")]

use dsp_cli::client::DspClient;
use dsp_cli::client::http::HttpDspClient;
use dsp_cli::config::Config;
use dsp_cli::model::RelationKind;

mod common;
use common::{optional_env, require_env};

// ---------------------------------------------------------------------------
// Live test
// ---------------------------------------------------------------------------

/// End-to-end live test: resolve project `0801` (beol), list its data-models,
/// find the `beol` data-model, then call `data_model_structure` and assert
/// structural invariants on the returned `DataModelStructure`.
///
/// `beol` is chosen because it is a well-known DaSCH research project with
/// cross-ontology link relations (e.g. `beol:letter` links to `biblio:Book`
/// via citation fields), which exercises both in-model and cross-model edges.
///
/// Skips cleanly (with an `eprintln!`) if the required `DSP_TEST_SERVER`
/// environment variable is absent. Never fails due to missing config — only
/// due to real errors.
///
/// Assertions are intentionally resilient to real-data variation:
/// - `data_model` field equals `"beol"`.
/// - At least one `Link` relation is returned (beol has link fields).
/// - At least one `Inherits` relation is returned (beol types inherit from each other, e.g.
///   `letter` extends `writtenSource`).
/// - At least one cross-model relation (`target_data_model = Some(...)` where the target dm differs
///   from `"beol"`) is returned — beol has link fields pointing to sibling data-models such as
///   `biblio`.
/// - All relation `source` and `target` names are non-empty.
/// - `field` is `Some` for every `Link` relation and `None` for every `Inherits` relation
///   (structural invariant from the domain model).
#[test]
#[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]
fn live_data_model_structure_returns_valid_structure() {
    // ── 1. Collect required config ────────────────────────────────────────────
    let server_raw = match require_env("DSP_TEST_SERVER") {
        Some(v) => v,
        None => return,
    };

    // Resolve server shortcut via Config::resolve (mirrors production path).
    let cfg = Config::resolve(Some(server_raw.trim())).expect("DSP_TEST_SERVER must be a valid server URL or shortcut");

    // ── 2. Resolve optional token ─────────────────────────────────────────────
    // `data_model_structure` uses a public endpoint; the token is optional. If
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
    // beol is chosen for its cross-ontology link relations (e.g. beol:letter →
    // biblio:Book), which exercise both in-model and cross-model edges.
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

    // ── 6. Call data_model_structure ─────────────────────────────────────────
    eprintln!("live test: calling data_model_structure for IRI {}", beol_dm.iri);

    let structure = client.data_model_structure(&cfg.server, &beol_dm.iri, token_ref).expect(
        "data_model_structure failed for beol — check DSP_TEST_SERVER and network \
             connectivity; if the beol data-model has been removed or restructured, update \
             this test to use a different project with cross-ontology link relations",
    );

    eprintln!(
        "live test: received structure for '{}' ({} relation(s))",
        structure.data_model,
        structure.relations.len()
    );

    // ── 7. Assert structural invariants ──────────────────────────────────────

    // data_model must be "beol" — we resolved this data-model by name.
    assert_eq!(
        structure.data_model, "beol",
        "structure.data_model must be 'beol'; got '{}'",
        structure.data_model
    );

    // relations must be non-empty — beol is a well-populated data-model.
    assert!(
        !structure.relations.is_empty(),
        "data-model 'beol' must have at least one relation (including builtins); got an \
         empty list. Check that DSP_TEST_SERVER points to a server where the beol \
         data-model is populated."
    );

    eprintln!(
        "live test: total relation count (incl. builtins): {}",
        structure.relations.len()
    );

    // Assert field/kind invariant: field is Some iff kind == Link.
    for rel in &structure.relations {
        match rel.kind {
            RelationKind::Link => {
                assert!(
                    rel.field.is_some(),
                    "Link relation must have a field; source='{}', target='{}'",
                    rel.source,
                    rel.target
                );
            }
            RelationKind::Inherits => {
                assert!(
                    rel.field.is_none(),
                    "Inherits relation must not have a field; source='{}', target='{}'",
                    rel.source,
                    rel.target
                );
            }
        }
        assert!(
            !rel.source.is_empty(),
            "relation source must not be empty (target: '{}', kind: {})",
            rel.target,
            rel.kind
        );
        assert!(
            !rel.target.is_empty(),
            "relation target must not be empty (source: '{}', kind: {})",
            rel.source,
            rel.kind
        );
    }

    // At least one Link relation must be present — beol defines link fields.
    let link_count = structure.relations.iter().filter(|r| r.kind == RelationKind::Link).count();
    assert!(
        link_count > 0,
        "expected at least one Link relation in beol but found none. \
         If beol no longer defines any link fields, update this assertion."
    );
    eprintln!("live test: link relation count: {link_count} — OK");

    // At least one Inherits relation must be present — beol types inherit from
    // each other (e.g. beol:letter extends beol:writtenSource).
    let inherits_count = structure.relations.iter().filter(|r| r.kind == RelationKind::Inherits).count();
    assert!(
        inherits_count > 0,
        "expected at least one Inherits relation in beol but found none. \
         If beol no longer has inheritance between resource-types, update this assertion."
    );
    eprintln!("live test: inherits relation count: {inherits_count} — OK");

    // At least one cross-model relation must be present — beol has link fields
    // pointing to sibling data-models such as `biblio` (e.g. beol:letter →
    // biblio:Book via citation-related fields).
    let cross_model_count = structure
        .relations
        .iter()
        .filter(|r| r.target_data_model.as_deref().is_some_and(|tdm| tdm != "beol"))
        .count();
    assert!(
        cross_model_count > 0,
        "expected at least one cross-model relation (target_data_model != 'beol') in beol \
         but found none. beol is known to have link fields pointing to sibling data-models \
         such as 'biblio'. If this is no longer the case, update this assertion."
    );
    eprintln!("live test: cross-model relation count: {cross_model_count} — OK");

    // Log a sample of relations for debugging.
    for rel in structure.relations.iter().take(10) {
        eprintln!(
            "live test: relation source='{}' target='{}' kind={} field={:?} \
             target_data_model={:?} is_builtin={}",
            rel.source, rel.target, rel.kind, rel.field, rel.target_data_model, rel.is_builtin
        );
    }
    if structure.relations.len() > 10 {
        eprintln!("live test: ... ({} more relations not shown)", structure.relations.len() - 10);
    }

    eprintln!(
        "live test: PASSED — data-model '{}' has {} relation(s) \
         ({} link, {} inherits, {} cross-model)",
        structure.data_model,
        structure.relations.len(),
        link_count,
        inherits_count,
        cross_model_count
    );
}
