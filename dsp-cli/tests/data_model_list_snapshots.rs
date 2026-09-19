//! Snapshot tests for `dsp vre data-model list` — one per (noun, format) cell.
//!
//! Fixture philosophy:
//! - **Main fixture** (4 project data-models): exercises the Option matrix (label+date,
//!   label+no-date, no-label+date, no-label+no-date), all `is_builtin: false`. Shared by prose,
//!   json, lines, csv, tsv cells.
//! - **Builtins fixture** (2 project + 3 builtins): `limc` and `rosetta` sort *between* the
//!   builtins, so `knora-api, limc, rosetta, salsah-gui, standoff` is the observable interleave
//!   order. Built in pre-sorted order. Shared by prose, json, csv, tsv. (Lines omits `is_builtin` —
//!   its builtins cell would add nothing new over the main fixture cell, so it is skipped.)
//! - **Empty fixture**: zero items, total 0. Prose and json only.
//! - **Filter fixture**: `filter: Some("...")` with `total > items.len()` so the prose header shows
//!   "(m of total matching …)". Prose only.
//! - **not_found json**: `renderer.diagnostic(Diagnostic::NotFound(…), &meta)` on a `JsonRenderer`
//!   — locks the dsp-cli/ADR-0012 propagated-error envelope.
//!
//! Determinism: these tests call `Renderer::data_models(&view, &meta)` (or
//! `renderer.diagnostic(…)`) **directly** with a hand-built `MetaContext` /
//! `DataModelListView`. They never go through `run_list_impl`, which reads the
//! real `DSP_TOKEN` env var. Action / auth-resolution logic is covered by the
//! in-module action tests; these layer-4 snapshot tests cover rendering only.
//! See `docs/src/dsp-cli/testing-strategy.md` and learning from plan 010.
//!
//! dsp-cli/ADR-0001 vocabulary guard: no `ontology`/`export`/`class`/`property` in this
//! file or any .snap it generates. (IRI strings contain "/ontology/" as data —
//! the documented exception per review-guidelines.md.)

use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::DataModel;
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{DataModelListView, MetaContext, Renderer};

mod support;
use support::{buf_to_string, shared_buf};

// ── server constant ───────────────────────────────────────────────────────────

/// Standard server label for all snapshot tests — mirrors what `run_list_impl`
/// sets from `cfg.server`.
const SERVER: &str = "api.dasch.swiss";

// ── MetaContext helpers ────────────────────────────────────────────────────────

/// Anonymous `MetaContext` — mirrors what `run_list_impl` builds when no token.
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

/// Helper to construct a `DataModel`.
fn dm(name: &str, iri: &str, label: Option<&str>, last_modified: Option<&str>, is_builtin: bool) -> DataModel {
    DataModel {
        name: name.to_string(),
        iri: iri.to_string(),
        label: label.map(Into::into),
        last_modified: last_modified.map(Into::into),
        is_builtin,
    }
}

/// Main fixture — 4 project data-models exercising every Option cell:
///   (label Some + date Some), (label Some + date None),
///   (label None + date Some), (label None + date None).
/// All `is_builtin: false`. Pre-sorted by name (action sorts before handing
/// to renderer). Realistic IRIs from the BEOL project (0801).
fn main_view() -> DataModelListView {
    let items = vec![
        dm(
            "beol",
            "http://api.dasch.swiss/ontology/0801/beol/v2",
            Some("The BEOL data-model"),
            Some("2024-05-27T13:43:26.233048Z"),
            false,
        ),
        dm(
            "biblio",
            "http://api.dasch.swiss/ontology/0801/biblio/v2",
            Some("Bibliographic references"),
            None,
            false,
        ),
        dm(
            "leibniz",
            "http://api.dasch.swiss/ontology/0801/leibniz/v2",
            None,
            Some("2023-11-14T09:15:00.000000Z"),
            false,
        ),
        dm("newton", "http://api.dasch.swiss/ontology/0801/newton/v2", None, None, false),
    ];
    let total = items.len();
    DataModelListView { items, total, filter: None }
}

/// Builtins fixture — 2 project data-models (`limc`, `rosetta`) interleaved
/// with the 3 platform builtins (`knora-api`, `salsah-gui`, `standoff`).
/// Built pre-sorted by name so the sort is `knora-api, limc, rosetta,
/// salsah-gui, standoff` — the interleave is observable (project names sit
/// *between* the builtins alphabetically).
fn builtins_view() -> DataModelListView {
    let items = vec![
        dm("knora-api", "http://api.knora.org/ontology/knora-api/v2", None, None, true),
        dm(
            "limc",
            "http://api.dasch.swiss/ontology/0897/limc/v2",
            Some("Lexicon Iconographicum Mythologiae Classicae"),
            Some("2024-02-10T11:00:00.000000Z"),
            false,
        ),
        dm(
            "rosetta",
            "http://api.dasch.swiss/ontology/0838/rosetta/v2",
            Some("Rosetta Stone project"),
            Some("2023-08-01T07:30:00.000000Z"),
            false,
        ),
        dm("salsah-gui", "http://api.knora.org/ontology/salsah-gui/v2", None, None, true),
        dm("standoff", "http://api.knora.org/ontology/standoff/v2", None, None, true),
    ];
    let total = items.len();
    DataModelListView { items, total, filter: None }
}

/// Empty fixture — zero items, total 0.
fn empty_view() -> DataModelListView {
    DataModelListView { items: vec![], total: 0, filter: None }
}

/// Filter fixture — 1 of 4 items survives the filter (only "beol" by name),
/// so prose shows "(1 of 4 matching \"beol\")".
fn filter_view() -> DataModelListView {
    let all = main_view().items;
    let total = 4; // full count before filter
    let filter = "beol";
    let needle = filter.to_lowercase();
    let items: Vec<DataModel> = all
        .into_iter()
        .filter(|d| {
            d.name.to_lowercase().contains(&needle) || d.label.as_deref().unwrap_or("").to_lowercase().contains(&needle)
        })
        .collect();
    // Items from main_view are already sorted; retain order.
    DataModelListView { items, total, filter: Some(filter.to_string()) }
}

// ── main fixture × 5 formats (anonymous) ─────────────────────────────────────

/// Prose render of the main fixture. Locks the aligned column layout,
/// date-only display (no time component), label vs empty slots,
/// no `(built-in)` marker, and the dsp-cli/ADR-0007 footer.
#[test]
fn data_model_list_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.data_models(&main_view(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// JSON render of the main fixture. Locks the `_meta`-first envelope,
/// `"data"` as an array, per-item key order (name/iri/label/last_modified/
/// is_builtin), label/last_modified as string or null, is_builtin as boolean.
#[test]
fn data_model_list_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.data_models(&main_view(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// Lines render of the main fixture. Locks the `name\tiri` per-row shape
/// (no header, no label/date/is_builtin columns — lean chaining format)
/// and that disclosure goes to stderr.
#[test]
fn data_model_list_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.data_models(&main_view(), &anon_meta()).unwrap();
    // Snapshot stdout (data rows only — no header per lines format).
    insta::assert_snapshot!("data_model_list_lines_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("data_model_list_lines_stderr", buf_to_string(&err_buf));
}

/// CSV render of the main fixture. Locks the header
/// `name,iri,label,last_modified,is_builtin`, per-row values (empty for None
/// Options), `false` for is_builtin, and that disclosure goes to stderr.
#[test]
fn data_model_list_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.data_models(&main_view(), &anon_meta()).unwrap();
    // Snapshot stdout (header + data rows).
    insta::assert_snapshot!("data_model_list_csv_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("data_model_list_csv_stderr", buf_to_string(&err_buf));
}

/// TSV render of the main fixture. Same columns as CSV but tab-separated
/// and unquoted. Disclosure goes to stderr.
#[test]
fn data_model_list_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.data_models(&main_view(), &anon_meta()).unwrap();
    // Snapshot stdout (header + data rows).
    insta::assert_snapshot!("data_model_list_tsv_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("data_model_list_tsv_stderr", buf_to_string(&err_buf));
}

// ── builtins fixture × 4 formats ─────────────────────────────────────────────
//
// Lines is skipped because it omits `is_builtin` — its builtins cell would be
// indistinguishable from the main fixture's lines cell (name+iri only).

/// Prose render of the builtins fixture. Locks:
/// - `, incl. built-ins` in the header.
/// - `(built-in)` label marker for the three platform builtins.
/// - Interleaved sort: `knora-api, limc, rosetta, salsah-gui, standoff`.
#[test]
fn data_model_list_builtins_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.data_models(&builtins_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    // Structural assertions before snapshotting.
    assert!(
        out.contains(", incl. built-ins"),
        "prose builtins header must contain ', incl. built-ins'; got:\n{out}"
    );
    assert!(
        out.contains("(built-in)"),
        "prose builtins rows must contain '(built-in)' marker; got:\n{out}"
    );
    // Check interleave: limc must appear between knora-api and salsah-gui.
    let lines: Vec<&str> = out.lines().collect();
    let knora_pos = lines.iter().position(|l| l.contains("knora-api")).unwrap();
    let limc_pos = lines.iter().position(|l| l.contains("limc")).unwrap();
    let salsah_pos = lines.iter().position(|l| l.contains("salsah-gui")).unwrap();
    let rosetta_pos = lines.iter().position(|l| l.contains("rosetta")).unwrap();
    let standoff_pos = lines.iter().position(|l| l.contains("standoff")).unwrap();
    assert!(knora_pos < limc_pos, "knora-api must appear before limc in sorted output");
    assert!(limc_pos < rosetta_pos, "limc must appear before rosetta in sorted output");
    assert!(
        rosetta_pos < salsah_pos,
        "rosetta must appear before salsah-gui in sorted output"
    );
    assert!(
        salsah_pos < standoff_pos,
        "salsah-gui must appear before standoff in sorted output"
    );
    insta::assert_snapshot!(out);
}

/// JSON render of the builtins fixture. Locks `is_builtin: true` as a JSON
/// boolean for the platform builtins and `label: null` for them, while the
/// project data-models have `is_builtin: false` and their labels/dates set.
#[test]
fn data_model_list_builtins_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.data_models(&builtins_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    // Structural assertion: is_builtin boolean.
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("builtins json must be valid JSON");
    let data = parsed["data"].as_array().unwrap();
    // knora-api is first (sorted): is_builtin true
    assert_eq!(data[0]["name"], "knora-api", "first item must be knora-api (sort order)");
    assert_eq!(data[0]["is_builtin"], true, "knora-api must have is_builtin: true");
    assert!(data[0]["label"].is_null(), "knora-api label must be null");
    // limc is second: is_builtin false
    assert_eq!(data[1]["name"], "limc", "second item must be limc (sort order)");
    assert_eq!(data[1]["is_builtin"], false, "limc must have is_builtin: false");
    insta::assert_snapshot!(out);
}

/// CSV render of the builtins fixture. Locks `true`/`false` strings in the
/// `is_builtin` column, empty label/last_modified for builtins, and the
/// interleaved sort visible in row order.
#[test]
fn data_model_list_builtins_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.data_models(&builtins_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    // Structural: header present, true/false for is_builtin.
    assert!(
        stdout.starts_with("name,iri,label,last_modified,is_builtin\n"),
        "CSV header must be name,iri,label,last_modified,is_builtin; got:\n{stdout}"
    );
    assert!(
        stdout.contains(",true"),
        "CSV builtins rows must contain ',true'; got:\n{stdout}"
    );
    assert!(
        stdout.contains(",false"),
        "CSV builtins rows must contain ',false'; got:\n{stdout}"
    );
    insta::assert_snapshot!("data_model_list_builtins_csv_stdout", stdout);
    insta::assert_snapshot!("data_model_list_builtins_csv_stderr", buf_to_string(&err_buf));
}

/// TSV render of the builtins fixture. Same column shape as CSV but
/// tab-separated and unquoted. Locks `true`/`false` in the is_builtin column.
#[test]
fn data_model_list_builtins_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.data_models(&builtins_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    assert!(
        stdout.starts_with("name\tiri\tlabel\tlast_modified\tis_builtin\n"),
        "TSV header must be name\\tiri\\tlabel\\tlast_modified\\tis_builtin; got:\n{stdout}"
    );
    assert!(
        stdout.contains("\ttrue"),
        "TSV builtins rows must contain '\\ttrue'; got:\n{stdout}"
    );
    assert!(
        stdout.contains("\tfalse"),
        "TSV builtins rows must contain '\\tfalse'; got:\n{stdout}"
    );
    insta::assert_snapshot!("data_model_list_builtins_tsv_stdout", stdout);
    insta::assert_snapshot!("data_model_list_builtins_tsv_stderr", buf_to_string(&err_buf));
}

// ── empty fixture × prose + json ──────────────────────────────────────────────

/// Prose render of the empty fixture. Locks the `(0):` header shape and the
/// dsp-cli/ADR-0007 footer with zero rows between them.
#[test]
fn data_model_list_prose_empty() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.data_models(&empty_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(out.contains("(0)"), "prose empty must show '(0)' count; got:\n{out}");
    assert!(
        out.contains("[anonymous on"),
        "prose empty must still have disclosure footer; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON render of the empty fixture. Locks `"data":[]` with the standard
/// `_meta` block.
#[test]
fn data_model_list_json_empty() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.data_models(&empty_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("empty json must be valid JSON");
    assert!(
        parsed["data"].as_array().unwrap().is_empty(),
        "json empty must have empty data array; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

// ── filter fixture × prose ────────────────────────────────────────────────────

/// Prose render with an active filter. The main fixture has 4 items; filtering
/// by "beol" yields 1 match (only the `beol` name itself — "biblio"'s label
/// "Bibliographic references" does not contain "beol"). Total stays at 4.
/// Locks the "(1 of 4 matching \"beol\")" count line and no `incl. built-ins`
/// (no builtins in the filtered set).
#[test]
fn data_model_list_prose_filter() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    let view = filter_view();
    r.data_models(&view, &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("matching \"beol\""),
        "prose filter must contain 'matching \"beol\"'; got:\n{out}"
    );
    assert!(out.contains("of 4"), "prose filter must show 'of 4' total; got:\n{out}");
    insta::assert_snapshot!(out);
}

// ── not_found JSON error envelope ─────────────────────────────────────────────

/// Snapshot of the `not_found` JSON error envelope produced by
/// `renderer.diagnostic(Diagnostic::NotFound(…), &meta)` on a `JsonRenderer`.
///
/// This locks the dsp-cli/ADR-0012 JSON envelope shape — the error path for when
/// `resolve_project` returns `NotFound` and it propagates through the action
/// without ever reaching `data_models`.
#[test]
fn data_model_list_json_not_found() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    let diag = Diagnostic::NotFound(
        "project '9999' not found on api.dasch.swiss. \
         Run `dsp vre project list --server api.dasch.swiss` to see available projects."
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

// ── stderr/stdout contract assertions (non-snapshot) ─────────────────────────

/// Lines: disclosure lands on stderr, NOT on stdout.
#[test]
fn data_model_list_lines_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.data_models(&main_view(), &anon_meta()).unwrap();
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
fn data_model_list_csv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.data_models(&main_view(), &anon_meta()).unwrap();
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
fn data_model_list_tsv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.data_models(&main_view(), &anon_meta()).unwrap();
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

/// Prose output must not contain vocabulary-leaked words from the DSP-API layer
/// (except inside IRI strings, which are data and the documented exception).
/// Note: checking for "ontolog" would be a false positive since IRIs in prose
/// are NOT rendered (dsp-cli/ADR-0003) — but JSON/CSV/TSV do render IRIs as data.
/// We check prose only (no IRI rendered) and check that the word "export" never
/// appears anywhere.
#[test]
fn data_model_list_prose_no_vocabulary_leak() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.data_models(&main_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.to_lowercase().contains("export"),
        "prose must not contain 'export'; got:\n{out}"
    );
    // Prose renders name + label + date — no IRIs — so "ontolog" must not appear.
    assert!(
        !out.to_lowercase().contains("ontolog"),
        "prose must not contain 'ontolog' (IRIs are not rendered in prose); got:\n{out}"
    );
    // dsp-cli/ADR-0001 banned terms: "class" and "property" must not appear in user-facing
    // prose data rows (DSP-API vocabulary must not leak through the translation
    // boundary). The fixture labels ("The BEOL data-model", "Bibliographic
    // references") do not contain these words, so this is safe to assert verbatim.
    assert!(
        !out.to_lowercase().contains("class"),
        "prose must not contain 'class' (DSP-API vocabulary leak); got:\n{out}"
    );
    assert!(
        !out.to_lowercase().contains("property"),
        "prose must not contain 'property' (DSP-API vocabulary leak); got:\n{out}"
    );
}

/// JSON output: "export" must not appear as a key or value in non-IRI data.
/// IRIs are data strings and are the documented exception for "/ontology/" in IRIs.
#[test]
fn data_model_list_json_no_export_leak() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.data_models(&main_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.to_lowercase().contains("export"),
        "json must not contain 'export'; got:\n{out}"
    );
}
