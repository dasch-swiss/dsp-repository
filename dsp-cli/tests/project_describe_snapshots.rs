//! Snapshot tests for `dsp vre project describe` — one per (noun, format) cell.
//!
//! Fixture philosophy: the five main-format cells share ONE realistic beol-shaped
//! fixture grounded in the live DSP API. Edge cases (empty fields) are kept in
//! their own dedicated fixture isolated from the main one so they never pollute
//! the other snapshots.
//!
//! Determinism: these tests call `Renderer::project_describe(&detail, &meta)`
//! directly with a hand-built `MetaContext`/`ProjectDetail` (the action
//! `run_describe_impl` is private and reads `DSP_TOKEN` from the real env).
//! The action path (auth resolution, cache fallback) is covered by the in-module
//! action tests; these snapshot tests cover rendering only.
//! See `docs/src/dsp-cli/testing-strategy.md` and the learning from plan 010.
//!
//! Main fixture (beol): realistic beol-shaped project grounded in the live API.
//! - IRI uses a random-suffix form (NOT shortcode-derived) as the real API does.
//! - 4 data-models (beol, biblio, leibniz, newton), pre-sorted by name.
//! - HTML markup in the description — rendered raw in prose (Risk 1).
//!
//! Edge fixture (testproject): `longname: None`, empty description, empty
//! keywords, zero data-models. Only tested with prose and json (the tabular
//! empty-cell behaviour is trivial and covered by the per-format unit tests).
//!
//! Error cell: `diagnostic` called directly on a JsonRenderer with a
//! `Diagnostic::NotFound` carrying the describe-specific recovery hint message.
//! Locks the dsp-cli/ADR-0012 JSON error envelope shape + the hint sentence.
//!
//! dsp-cli/ADR-0001 vocabulary guard: no `ontology`/`export`/`class`/`property` in this
//! file or any .snap it generates. (IRI strings contain "/ontology/" as data
//! — the documented exception per review-guidelines.md — but the word must not
//! appear as dsp-cli vocabulary in rendered labels or keys.)

use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::{DataModelSummary, ProjectDescription, ProjectDetail};
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{MetaContext, Renderer};

mod support;
use support::{buf_to_string, shared_buf};

// ── fixtures ──────────────────────────────────────────────────────────────────

/// Standard server label for all snapshot tests — mirrors what `run_describe_impl`
/// sets from `cfg.server`.
const SERVER: &str = "https://api.dasch.swiss";

/// Standard anonymous `MetaContext` — mirrors what `run_describe_impl` builds
/// when no token is present.
fn anon_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

/// Main fixture: beol-shaped project grounded in the live DSP API.
///
/// IRI is a realistic random-suffix IRI (NOT shortcode-derived: the real beol
/// project at api.dasch.swiss/admin/projects/shortcode/0801 returns exactly this
/// `id`). Data-models are pre-sorted by name (the HTTP client sorts them before
/// building `ProjectDetail`).
fn beol_detail() -> ProjectDetail {
    ProjectDetail {
        iri: "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF".to_string(),
        shortcode: "0801".to_string(),
        shortname: "beol".to_string(),
        longname: Some("Bernoulli-Euler Online".to_string()),
        description: vec![ProjectDescription {
            value: "<b>BEOL</b> — early modern mathematics.".to_string(),
            language: Some("en".to_string()),
        }],
        keywords: vec!["Bernoulli".to_string(), "Euler".to_string(), "Mathematics".to_string()],
        data_models: vec![
            DataModelSummary {
                name: "beol".to_string(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2".to_string(),
            },
            DataModelSummary {
                name: "biblio".to_string(),
                iri: "http://api.dasch.swiss/ontology/0801/biblio/v2".to_string(),
            },
            DataModelSummary {
                name: "leibniz".to_string(),
                iri: "http://api.dasch.swiss/ontology/0801/leibniz/v2".to_string(),
            },
            DataModelSummary {
                name: "newton".to_string(),
                iri: "http://api.dasch.swiss/ontology/0801/newton/v2".to_string(),
            },
        ],
    }
}

/// Edge fixture: a small project with no longname, no description, no keywords,
/// no data-models. Isolated from the main fixture so edge-rendering behaviour
/// (omitted lines, `Data-models (0)`, null/empty-array in json) is locked
/// without polluting the main snapshots.
fn edge_detail() -> ProjectDetail {
    ProjectDetail {
        iri: "http://rdfh.ch/projects/Kx9mL4rT2pWuB6nZ".to_string(),
        shortcode: "4123".to_string(),
        shortname: "testproject".to_string(),
        longname: None,
        description: vec![],
        keywords: vec![],
        data_models: vec![],
    }
}

// ── main fixture × 5 formats ──────────────────────────────────────────────────

/// Prose render of the beol fixture. Locks the label/value block layout,
/// the `Data-models (4): beol, biblio, leibniz, newton` line, the
/// HTML-converted description (plain text with links as `text (url)`,
/// tags stripped, control chars removed), and the dsp-cli/ADR-0007 footer.
/// Header is `Project: <shortname> (<shortcode>)`; longname is a `Name:` field.
#[test]
fn project_describe_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.project_describe(&beol_detail(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// JSON render of the beol fixture. Locks the single-object `data` shape
/// (dsp-cli/ADR-0003), the deterministic key order, `longname` as a string (not null),
/// the `description` array with `{value, language}` elements, the `keywords`
/// array, and the `data_models` array with `{name, iri}` elements.
#[test]
fn project_describe_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.project_describe(&beol_detail(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// Lines render of the beol fixture. Locks the single-row
/// `shortcode\tshortname\tlongname` shape (no header) and that the
/// disclosure goes to stderr (captured separately) not stdout.
#[test]
fn project_describe_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.project_describe(&beol_detail(), &anon_meta()).unwrap();
    // Snapshot stdout (data row only — no header per lines format).
    insta::assert_snapshot!("project_describe_lines_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("project_describe_lines_stderr", buf_to_string(&err_buf));
}

/// CSV render of the beol fixture. Locks the header + one data row shape,
/// `data_models` as the count 4 (names are prose/json-only), and that
/// the disclosure goes to stderr not stdout.
#[test]
fn project_describe_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.project_describe(&beol_detail(), &anon_meta()).unwrap();
    // Snapshot stdout (header + data row).
    insta::assert_snapshot!("project_describe_csv_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("project_describe_csv_stderr", buf_to_string(&err_buf));
}

/// TSV render of the beol fixture. Same column shape as CSV but tab-separated
/// and no quoting. `data_models` = count 4.
#[test]
fn project_describe_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.project_describe(&beol_detail(), &anon_meta()).unwrap();
    // Snapshot stdout (header + data row).
    insta::assert_snapshot!("project_describe_tsv_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("project_describe_tsv_stderr", buf_to_string(&err_buf));
}

// ── edge fixture × prose + json ───────────────────────────────────────────────

/// Prose render of the edge fixture (`longname: None`, empty keywords,
/// empty description, zero data-models). Locks:
/// - Header is `Project: testproject (4123)` (shortname + shortcode, always).
/// - `Name:` line is omitted when longname is None.
/// - `Keywords:` line is omitted entirely.
/// - `Description:` block is omitted entirely.
/// - `Data-models (0)` appears with no trailing names.
#[test]
fn project_describe_prose_edge() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.project_describe(&edge_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    // Structural assertions before snapshotting, for self-documenting failures.
    // New layout B: header is always "Project: <shortname> (<shortcode>)".
    assert!(
        out.contains("Project: testproject (4123)"),
        "prose edge: header must be 'Project: testproject (4123)'; got: {out}"
    );
    // Name: line must be absent when longname is None.
    assert!(
        !out.contains("Name:"),
        "prose edge: Name: line must be absent when longname is None; got: {out}"
    );
    assert!(
        !out.contains("Keywords"),
        "prose edge: Keywords line must be omitted when empty; got: {out}"
    );
    assert!(
        !out.contains("Description"),
        "prose edge: Description block must be omitted when empty; got: {out}"
    );
    assert!(
        out.contains("Data-models (0)"),
        "prose edge: must show 'Data-models (0)' for empty data-models; got: {out}"
    );
    assert!(
        !out.contains("Data-models (0):"),
        "prose edge: 'Data-models (0)' must have no trailing colon/names; got: {out}"
    );

    insta::assert_snapshot!(out);
}

/// JSON render of the edge fixture. Locks:
/// - `longname` is `null`.
/// - `description` is an empty array `[]`.
/// - `keywords` is an empty array `[]`.
/// - `data_models` is an empty array `[]`.
#[test]
fn project_describe_json_edge() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.project_describe(&edge_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    // Parse and assert structurally, then snapshot.
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("edge json must be valid JSON");
    assert!(
        parsed["data"]["longname"].is_null(),
        "json edge: longname must be null; got: {}",
        parsed["data"]["longname"]
    );
    assert!(
        parsed["data"]["description"].as_array().unwrap().is_empty(),
        "json edge: description must be empty array"
    );
    assert!(
        parsed["data"]["keywords"].as_array().unwrap().is_empty(),
        "json edge: keywords must be empty array"
    );
    assert!(
        parsed["data"]["data_models"].as_array().unwrap().is_empty(),
        "json edge: data_models must be empty array"
    );

    insta::assert_snapshot!(out);
}

// ── error cell: not_found JSON envelope ──────────────────────────────────────

/// Snapshot of the `not_found` JSON error envelope produced by
/// `renderer.diagnostic(Diagnostic::NotFound(…), &meta)` on a `JsonRenderer`.
///
/// This is the correct mechanism for the describe not-found path: the error
/// propagates from `client.describe_project(…)?` via `?` in `run_describe_impl`
/// and is rendered by the generic `diagnostic` method — `project_describe` never
/// sees it. Snapshotting here locks:
/// - The dsp-cli/ADR-0012 JSON envelope shape (`_meta.error.kind`, `_meta.error.message`).
/// - The describe-specific not-found message + the recovery hint sentence.
#[test]
fn project_describe_json_not_found() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    let diag = Diagnostic::NotFound(
        "project '9999' not found on https://api.dasch.swiss. \
         Run `dsp vre project list --server https://api.dasch.swiss` to see available projects."
            .to_string(),
    );
    r.diagnostic(&diag, &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    // Structural assertion before snapshotting.
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("not_found json must be valid JSON");
    assert_eq!(
        parsed["error"]["kind"], "not_found",
        "error envelope must have kind='not_found'; got: {}",
        parsed["error"]["kind"]
    );
    assert!(
        parsed["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("dsp vre project list"),
        "error message must include the recovery hint; got: {}",
        parsed["error"]["message"]
    );

    insta::assert_snapshot!(out);
}
