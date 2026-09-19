//! Snapshot tests for `dsp vre data-model describe` — one per (noun, format) cell.
//!
//! Fixture philosophy:
//! - **Main fixture** (beol-like data-model): exercises the Option matrix for label/last_modified
//!   (both Some), plus several real beol resource-types with mixed-case names and one with `label:
//!   None`. Shared by prose, json, lines, csv, tsv cells.
//! - **Empty fixture**: `name: "minimal"`, `label: None`, `last_modified: None`, `resource_types:
//!   vec![]`. Tested with prose, json, csv (locks the zero-count prose branch, json null fields,
//!   and csv zero-count row).
//! - **not_found json**: `renderer.diagnostic(Diagnostic::NotFound(…), &meta)` on a `JsonRenderer`
//!   — locks the action-built NotFound envelope rendered by the generic `diagnostic` method.
//!
//! Determinism: these tests call `Renderer::data_model_describe(&detail, &meta)`
//! **directly** with a hand-built `MetaContext` / `DataModelDetail`. They never go
//! through `run_describe_impl`, which reads the real `DSP_TOKEN` env var. Action /
//! auth-resolution logic is covered by the in-module action tests; these layer-4
//! snapshot tests cover rendering only.
//! See `docs/dev/testing-strategy.md` and learning from plan 010.
//!
//! ADR-0001 vocabulary guard: no `ontology`/`export`/`class`/`property` in this
//! file or any .snap it generates. (IRI strings contain "/ontology/" as data —
//! the documented exception per review-guidelines.md.)

use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::{DataModelDetail, ResourceTypeSummary};
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{MetaContext, Renderer};

mod support;
use support::{buf_to_string, shared_buf};

// ── server constant ───────────────────────────────────────────────────────────

/// Standard server label for all snapshot tests — mirrors what `run_describe_impl`
/// sets from `cfg.server`.
const SERVER: &str = "https://api.dasch.swiss";

// ── MetaContext helpers ────────────────────────────────────────────────────────

/// Anonymous `MetaContext` — mirrors what `run_describe_impl` builds when no
/// token is present.
fn anon_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

// ── fixtures ──────────────────────────────────────────────────────────────────

/// Helper to construct a `ResourceTypeSummary`.
fn rt(name: &str, iri: &str, label: Option<&str>) -> ResourceTypeSummary {
    ResourceTypeSummary {
        name: name.to_string(),
        iri: iri.to_string(),
        label: label.map(Into::into),
    }
}

/// Main fixture — a beol-like data-model exercising:
/// - `label: Some`, `last_modified: Some`
/// - Several real beol resource-types with mixed-case names (sorted by name)
/// - One resource-type with `label: None` (`letter`)
///
/// Real beol resource-type names and IRIs grounded in the live API.
/// Pre-sorted by name (the HTTP client sorts before building `DataModelDetail`).
fn beol_detail() -> DataModelDetail {
    DataModelDetail {
        name: "beol".to_string(),
        iri: "http://api.dasch.swiss/ontology/0801/beol/v2".to_string(),
        label: Some("The BEOL data-model".to_string()),
        last_modified: Some("2024-05-27T13:43:26.233048Z".to_string()),
        resource_types: vec![
            rt(
                "Archive",
                "http://api.dasch.swiss/ontology/0801/beol/v2#Archive",
                Some("Archive"),
            ),
            rt(
                "basicLetter",
                "http://api.dasch.swiss/ontology/0801/beol/v2#basicLetter",
                Some("Basic Letter"),
            ),
            rt("letter", "http://api.dasch.swiss/ontology/0801/beol/v2#letter", None),
            rt("person", "http://api.dasch.swiss/ontology/0801/beol/v2#person", Some("Person")),
        ],
    }
}

/// Empty fixture — zero resource-types, `label: None`, `last_modified: None`.
/// Locks the prose `Resource-types (0)` branch, json null fields, and csv
/// zero-count row with empty label/last_modified.
fn minimal_detail() -> DataModelDetail {
    DataModelDetail {
        name: "minimal".to_string(),
        iri: "http://api.dasch.swiss/ontology/0000/minimal/v2".to_string(),
        label: None,
        last_modified: None,
        resource_types: vec![],
    }
}

// ── main fixture × 5 formats ──────────────────────────────────────────────────

/// Prose render of the main fixture. Locks:
/// - Header `Data-model: beol`
/// - Label/IRI/Last-modified block (values aligned, date stripped to YYYY-MM-DD)
/// - `Resource-types (4):` sub-list with aligned `name  label` columns
/// - One resource-type with empty label (`letter`)
/// - ADR-0007 footer `[anonymous on https://api.dasch.swiss]`
#[test]
fn data_model_describe_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.data_model_describe(&beol_detail(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// JSON render of the main fixture. Locks:
/// - ADR-0003 single-object `data` envelope with `_meta` first
/// - Key order: name, iri, label, last_modified, resource_types
/// - `last_modified` as full RFC3339 string (lossless)
/// - `resource_types` array with per-resource-type {name, iri, label}
/// - `label: null` for the resource-type without a label
#[test]
fn data_model_describe_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.data_model_describe(&beol_detail(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// Lines render of the main fixture. Locks:
/// - Single row `name\tiri` (no header, no resource-types — lean chaining format)
/// - Disclosure on stderr, not stdout
#[test]
fn data_model_describe_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.data_model_describe(&beol_detail(), &anon_meta()).unwrap();
    // Snapshot stdout (data row only).
    insta::assert_snapshot!("data_model_describe_lines_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("data_model_describe_lines_stderr", buf_to_string(&err_buf));
}

/// CSV render of the main fixture. Locks:
/// - Header `name,iri,label,last_modified,resource_types`
/// - One data row with full RFC3339 last_modified, label string, count 4
/// - Disclosure on stderr, not stdout
#[test]
fn data_model_describe_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.data_model_describe(&beol_detail(), &anon_meta()).unwrap();
    // Snapshot stdout (header + data row).
    insta::assert_snapshot!("data_model_describe_csv_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("data_model_describe_csv_stderr", buf_to_string(&err_buf));
}

/// TSV render of the main fixture. Same column shape as CSV but tab-separated
/// and unquoted. `resource_types` = count 4. Disclosure on stderr.
#[test]
fn data_model_describe_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.data_model_describe(&beol_detail(), &anon_meta()).unwrap();
    // Snapshot stdout (header + data row).
    insta::assert_snapshot!("data_model_describe_tsv_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("data_model_describe_tsv_stderr", buf_to_string(&err_buf));
}

// ── empty fixture × prose + json + csv ────────────────────────────────────────

/// Prose render of the empty fixture. Locks:
/// - `Label:` line omitted when None
/// - `Last-modified:` line omitted when None
/// - `  Resource-types (0)` with NO trailing colon or sub-list
/// - ADR-0007 footer still present
#[test]
fn data_model_describe_prose_empty() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.data_model_describe(&minimal_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    // Structural assertions before snapshotting.
    assert!(
        out.contains("Resource-types (0)"),
        "prose empty must show 'Resource-types (0)'; got:\n{out}"
    );
    assert!(
        !out.contains("Resource-types (0):"),
        "prose empty must NOT have trailing colon on 'Resource-types (0)'; got:\n{out}"
    );
    assert!(
        !out.contains("Label:"),
        "prose empty must omit Label: when label is None; got:\n{out}"
    );
    assert!(
        !out.contains("Last-modified:"),
        "prose empty must omit Last-modified: when last_modified is None; got:\n{out}"
    );
    assert!(
        out.contains("[anonymous on"),
        "prose empty must still have disclosure footer; got:\n{out}"
    );

    insta::assert_snapshot!(out);
}

/// JSON render of the empty fixture. Locks:
/// - `label: null` and `last_modified: null`
/// - `resource_types: []`
#[test]
fn data_model_describe_json_empty() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.data_model_describe(&minimal_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("empty json must be valid JSON");
    assert!(
        parsed["data"]["label"].is_null(),
        "json empty: label must be null; got: {}",
        parsed["data"]["label"]
    );
    assert!(
        parsed["data"]["last_modified"].is_null(),
        "json empty: last_modified must be null; got: {}",
        parsed["data"]["last_modified"]
    );
    assert!(
        parsed["data"]["resource_types"].as_array().unwrap().is_empty(),
        "json empty: resource_types must be an empty array; got: {}",
        parsed["data"]["resource_types"]
    );

    insta::assert_snapshot!(out);
}

/// CSV render of the empty fixture. Locks the zero-count row:
/// `minimal,http://…/minimal/v2,,,0`
/// (empty label, empty last_modified, resource_types count = 0).
#[test]
fn data_model_describe_csv_empty() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.data_model_describe(&minimal_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);

    // Structural assertion: header present and count is 0.
    assert!(
        stdout.starts_with("name,iri,label,last_modified,resource_types\n"),
        "csv empty: header must be name,iri,label,last_modified,resource_types; got:\n{stdout}"
    );
    assert!(
        stdout.contains(",0\n"),
        "csv empty: resource_types count must be 0; got:\n{stdout}"
    );

    insta::assert_snapshot!("data_model_describe_csv_empty_stdout", stdout);
    insta::assert_snapshot!("data_model_describe_csv_empty_stderr", buf_to_string(&err_buf));
}

// ── not_found JSON error envelope ─────────────────────────────────────────────

/// Snapshot of the `not_found` JSON error envelope produced by
/// `renderer.diagnostic(Diagnostic::NotFound(…), &meta)` on a `JsonRenderer`.
///
/// This locks the ADR-0012 JSON envelope shape — the error path for when the
/// `--data-model` name/IRI does not match any data-model in the project. The
/// action builds the `NotFound` message with a recovery hint and propagates it
/// via `?`; `main.rs` calls `renderer.diagnostic(…)`.
#[test]
fn data_model_describe_json_not_found() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    let diag = Diagnostic::NotFound(
        "data-model 'xyz' not found in project '0801' on https://api.dasch.swiss. \
         Run `dsp vre data-model list --project 0801 --server https://api.dasch.swiss` \
         to see available data-models."
            .to_string(),
    );
    r.diagnostic(&diag, &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    // Structural assertions before snapshotting.
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
            .contains("dsp vre data-model list"),
        "error message must include the recovery hint; got: {}",
        parsed["error"]["message"]
    );

    insta::assert_snapshot!(out);
}

// ── stderr/stdout contract assertions (non-snapshot) ─────────────────────────

/// Lines: disclosure lands on stderr, NOT on stdout.
#[test]
fn data_model_describe_lines_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.data_model_describe(&beol_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    assert!(
        stderr.contains("[anonymous on"),
        "lines: disclosure must be on stderr; got stderr={stderr:?}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "lines: disclosure must NOT be on stdout; got stdout={stdout:?}"
    );
}

/// CSV: disclosure lands on stderr, NOT on stdout.
#[test]
fn data_model_describe_csv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.data_model_describe(&beol_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    assert!(
        stderr.contains("[anonymous on"),
        "csv: disclosure must be on stderr; got stderr={stderr:?}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "csv: disclosure must NOT be on stdout; got stdout={stdout:?}"
    );
}

/// TSV: disclosure lands on stderr, NOT on stdout.
#[test]
fn data_model_describe_tsv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.data_model_describe(&beol_detail(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    assert!(
        stderr.contains("[anonymous on"),
        "tsv: disclosure must be on stderr; got stderr={stderr:?}"
    );
    assert!(
        !stdout.contains("[anonymous on"),
        "tsv: disclosure must NOT be on stdout; got stdout={stdout:?}"
    );
}

// ── vocabulary guard (inline) ─────────────────────────────────────────────────

/// Prose output must not contain vocabulary-leaked words from the DSP-API layer.
/// Exception: the `IRI:` field is always rendered in `data-model describe` prose
/// (per the plan: "IRI: always"), and IRIs contain "/ontology/" as data — this
/// is the documented IRI-value exception per review-guidelines.md.
/// The word "export" must never appear anywhere; bare "class"/"property" must not
/// appear in labels or field keys (DSP-API vocabulary must not leak through the
/// translation boundary).
#[test]
fn data_model_describe_prose_no_vocabulary_leak() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.data_model_describe(&beol_detail(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);

    assert!(
        !out.to_lowercase().contains("export"),
        "prose must not contain 'export'; got:\n{out}"
    );
    // Prose renders `IRI: http://…/ontology/…` — so "/ontology/" appears as data
    // (the documented IRI-value exception). Checking that NO OTHER non-IRI usage
    // of the word "class" or "property" leaks.
    assert!(
        !out.to_lowercase().contains("class"),
        "prose must not contain 'class' (DSP-API vocabulary leak); got:\n{out}"
    );
    assert!(
        !out.to_lowercase().contains("property"),
        "prose must not contain 'property' (DSP-API vocabulary leak); got:\n{out}"
    );
}
