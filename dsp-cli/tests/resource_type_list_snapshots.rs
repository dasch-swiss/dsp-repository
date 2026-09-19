//! Snapshot tests for `dsp vre resource-type list` — one per (noun, format) cell.
//!
//! Fixture philosophy:
//! - **Main fixture** (3 project resource-types in BEOL): exercises the Option matrix (label Some,
//!   label Some, label None), all `is_builtin: false`. Shared by prose, json, lines, csv, tsv
//!   cells.
//! - **Builtins fixture** (2 project + 4 builtins): `Archive` and `letter` sort between the
//!   builtins alphabetically: `Archive, AudioSegment, letter, LinkObj, Region, VideoSegment`.
//!   Shared by prose, json, csv, tsv. (Lines omits `is_builtin` — its builtins cell would add
//!   nothing new over the main fixture's lines cell, so it is skipped.)
//! - **Empty fixture**: zero items, total 0. Prose and json only.
//! - **Filter fixture**: `filter: Some("let")` with `total > items.len()` so the prose header shows
//!   "(m of total matching …)". Prose only.
//! - **not_found json**: `renderer.diagnostic(Diagnostic::NotFound(…), &meta)` on a `JsonRenderer`
//!   — locks the dsp-cli/ADR-0012 propagated-error envelope for the data-model-not-found path.
//!
//! Determinism: these tests call `Renderer::resource_types(&view, &meta)` (or
//! `renderer.diagnostic(…)`) **directly** with a hand-built `MetaContext` /
//! `ResourceTypeListView`. They never go through `run_list_impl`, which reads the
//! real `DSP_TOKEN` env var. Action / auth-resolution logic is covered by the
//! in-module action tests; these layer-4 snapshot tests cover rendering only.
//! See `docs/src/dsp-cli/testing-strategy.md` and dsp-cli/ADR-0009.
//!
//! Vocabulary guard: no `export`/`class`/`property` in this file or any .snap it
//! generates. NOTE: checking for `ontolog` would be a false positive for IRI strings
//! (e.g. `http://api.dasch.swiss/ontology/0801/beol/v2#letter`) — those are data
//! values, the documented exception. Only prose (which does not render IRIs) gets
//! the `ontolog` check; tabular/json formats render raw IRIs so the `ontolog` check
//! is explicitly skipped there. (dsp-cli/ADR-0001 / Step-5 learning from plan 014.)

use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::model::ResourceType;
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{MetaContext, Renderer, ResourceTypeListView};

mod support;
use support::{buf_to_string, shared_buf};

// ── server constant ───────────────────────────────────────────────────────────

/// Standard server label for all snapshot tests — mirrors what `run_list_impl`
/// sets from `cfg.server`.
const SERVER: &str = "api.dasch.swiss";

// ── MetaContext helpers ───────────────────────────────────────────────────────

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

/// Anonymous `MetaContext` with `count_caveat` set — mirrors what
/// `run_list_impl` builds when `--count` is passed. Wording is copied
/// verbatim from `COUNT_CAVEAT` in `src/actions/vre/resource_type.rs` so the
/// two stay in sync.
fn count_caveat_meta() -> MetaContext {
    MetaContext {
        count_caveat: Some(
            "counts include resources you may not be permitted to see and exclude deleted resources.".to_string(),
        ),
        ..anon_meta()
    }
}

// ── fixtures ──────────────────────────────────────────────────────────────────

/// Helper to construct a `ResourceType`.
fn rt(name: &str, iri: &str, label: Option<&str>, is_builtin: bool) -> ResourceType {
    ResourceType {
        name: name.to_string(),
        iri: iri.to_string(),
        label: label.map(Into::into),
        is_builtin,
        count: None,
    }
}

/// Main fixture — 3 project resource-types from the BEOL data-model:
///   `Archive` (label Some), `letter` (label Some), `noLabelType` (label None).
/// All `is_builtin: false`. Pre-sorted by name (action sorts before handing
/// to renderer). Realistic IRIs from the BEOL project (0801).
fn main_view() -> ResourceTypeListView {
    let items = vec![
        rt(
            "Archive",
            "http://api.dasch.swiss/ontology/0801/beol/v2#Archive",
            Some("Archive"),
            false,
        ),
        rt(
            "letter",
            "http://api.dasch.swiss/ontology/0801/beol/v2#letter",
            Some("Letter"),
            false,
        ),
        rt(
            "noLabelType",
            "http://api.dasch.swiss/ontology/0801/beol/v2#noLabelType",
            None,
            false,
        ),
    ];
    let total = items.len();
    ResourceTypeListView { items, total, filter: None, data_model: "beol".to_string() }
}

/// Main fixture with `--count` merged in — the same 3 project resource-types
/// as `main_view()`, but with `count` populated: `Archive: Some(3)`,
/// `letter: Some(150)`, `noLabelType: None` (deliberately left absent, mirroring
/// the "class absent from the v3 count map" case exercised at the action layer
/// by `test_list_count_absent_class_is_none_not_error` in
/// `src/actions/vre/resource_type.rs`). Exercises both "present" and
/// "absent/blank" count rendering in one fixture.
fn main_view_with_counts() -> ResourceTypeListView {
    let mut view = main_view();
    for item in &mut view.items {
        item.count = match item.name.as_str() {
            "Archive" => Some(3),
            "letter" => Some(150),
            _ => None,
        };
    }
    view
}

/// Builtins fixture — 2 project resource-types (`Archive`, `letter`) interleaved
/// with the 4 platform builtins (`AudioSegment`, `LinkObj`, `Region`,
/// `VideoSegment`).
/// Pre-sorted by name so the interleave is observable:
///   `Archive, AudioSegment, letter, LinkObj, Region, VideoSegment`.
fn builtins_view() -> ResourceTypeListView {
    let items = vec![
        rt(
            "Archive",
            "http://api.dasch.swiss/ontology/0801/beol/v2#Archive",
            Some("Archive"),
            false,
        ),
        rt(
            "AudioSegment",
            "http://api.knora.org/ontology/knora-api/v2#AudioSegment",
            Some("Audio Annotation"),
            true,
        ),
        rt(
            "letter",
            "http://api.dasch.swiss/ontology/0801/beol/v2#letter",
            Some("Letter"),
            false,
        ),
        rt(
            "LinkObj",
            "http://api.knora.org/ontology/knora-api/v2#LinkObj",
            Some("Link Object"),
            true,
        ),
        rt(
            "Region",
            "http://api.knora.org/ontology/knora-api/v2#Region",
            Some("Region"),
            true,
        ),
        rt(
            "VideoSegment",
            "http://api.knora.org/ontology/knora-api/v2#VideoSegment",
            Some("Video Annotation"),
            true,
        ),
    ];
    let total = items.len();
    ResourceTypeListView { items, total, filter: None, data_model: "beol".to_string() }
}

/// Empty fixture — zero items, total 0.
fn empty_view() -> ResourceTypeListView {
    ResourceTypeListView {
        items: vec![],
        total: 0,
        filter: None,
        data_model: "beol".to_string(),
    }
}

/// Filter fixture — 1 of 3 items survives the filter ("let" matches only `letter`
/// via its name), so prose shows "(1 of 3 matching \"let\")".
fn filter_view() -> ResourceTypeListView {
    let all = main_view().items;
    let total = 3; // full count before filter
    let filter = "let";
    let needle = filter.to_lowercase();
    let items: Vec<ResourceType> = all
        .into_iter()
        .filter(|rt| {
            rt.name.to_lowercase().contains(&needle)
                || rt.label.as_deref().unwrap_or("").to_lowercase().contains(&needle)
        })
        .collect();
    ResourceTypeListView {
        items,
        total,
        filter: Some(filter.to_string()),
        data_model: "beol".to_string(),
    }
}

// ── main fixture × 5 formats (anonymous) ─────────────────────────────────────

/// Prose render of the main fixture. Locks the aligned column layout, label vs
/// empty slots, no `(built-in)` marker on project-only items, and the dsp-cli/ADR-0007
/// footer.
#[test]
fn resource_type_list_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_types(&main_view(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// JSON render of the main fixture. Locks the `_meta`-first envelope, `"data"` as
/// an array, per-item key order (name/iri/label/is_builtin), label as string or
/// null, is_builtin as boolean false.
#[test]
fn resource_type_list_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_types(&main_view(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// Lines render of the main fixture. Locks the `name\tiri` per-row shape (no
/// header, no label/is_builtin columns — lean chaining format) and that disclosure
/// goes to stderr.
#[test]
fn resource_type_list_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resource_types(&main_view(), &anon_meta()).unwrap();
    // Snapshot stdout (data rows only — no header per lines format).
    insta::assert_snapshot!("resource_type_list_lines_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("resource_type_list_lines_stderr", buf_to_string(&err_buf));
}

/// CSV render of the main fixture (project-only, with builtins). Locks the header
/// `name,iri,label,is_builtin` (no last_modified — resource-types have none),
/// per-row values (empty for None label), `false` for is_builtin, and that
/// disclosure goes to stderr.
#[test]
fn resource_type_list_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_types(&main_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    // Structural: header present.
    assert!(
        stdout.starts_with("name,iri,label,is_builtin\n"),
        "CSV header must be name,iri,label,is_builtin; got:\n{stdout}"
    );
    // Snapshot stdout (header + data rows).
    insta::assert_snapshot!("resource_type_list_csv_stdout", stdout);
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("resource_type_list_csv_stderr", buf_to_string(&err_buf));
}

/// TSV render of the main fixture. Same columns as CSV but tab-separated and
/// unquoted. Disclosure goes to stderr.
#[test]
fn resource_type_list_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_types(&main_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    // Structural: header present.
    assert!(
        stdout.starts_with("name\tiri\tlabel\tis_builtin\n"),
        "TSV header must be name\\tiri\\tlabel\\tis_builtin; got:\n{stdout}"
    );
    // Snapshot stdout (header + data rows).
    insta::assert_snapshot!("resource_type_list_tsv_stdout", stdout);
    // Snapshot stderr: disclosure must land here, not on stdout.
    insta::assert_snapshot!("resource_type_list_tsv_stderr", buf_to_string(&err_buf));
}

// ── main fixture × 5 formats, with `--count` (plan 030) ──────────────────────

/// Prose render of the main fixture with `--count`. Locks the right-aligned
/// count column (populated for Archive/letter, blank for noLabelType) and the
/// count_caveat appended to the dsp-cli/ADR-0007 footer.
#[test]
fn resource_type_list_prose_with_count() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_types(&main_view_with_counts(), &count_caveat_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// JSON render of the main fixture with `--count`. Locks the `count` key
/// present (as a number) for items that carry one, absent (not null) for
/// `noLabelType`, and `_meta.note` carrying the count_caveat text.
#[test]
fn resource_type_list_json_with_count() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_types(&main_view_with_counts(), &count_caveat_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// Lines render of the main fixture with `--count`. `count` is never part of
/// the lines default column set (see `src/render/lines.rs`), so stdout is
/// unaffected by `--count`; only the stderr disclosure line gains the
/// count_caveat suffix.
#[test]
fn resource_type_list_lines_with_count() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resource_types(&main_view_with_counts(), &count_caveat_meta()).unwrap();
    // Snapshot stdout (data rows only — no header per lines format).
    insta::assert_snapshot!("resource_type_list_lines_with_count_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure now carries the count_caveat text.
    insta::assert_snapshot!("resource_type_list_lines_with_count_stderr", buf_to_string(&err_buf));
}

/// CSV render of the main fixture with `--count`. The `count` column
/// auto-shows once at least one item carries a count (5-column header:
/// `name,iri,label,is_builtin,count` — see the `default_columns` logic in
/// `src/render/csv.rs`). No hardcoded 4-column header assertion here — that
/// belongs to the unflagged `resource_type_list_csv` test above; the 5-column
/// shape is captured by the snapshot itself.
#[test]
fn resource_type_list_csv_with_count() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_types(&main_view_with_counts(), &count_caveat_meta()).unwrap();
    // Snapshot stdout (header + data rows).
    insta::assert_snapshot!("resource_type_list_csv_with_count_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout, and must now
    // carry the count_caveat text.
    insta::assert_snapshot!("resource_type_list_csv_with_count_stderr", buf_to_string(&err_buf));
}

/// TSV render of the main fixture with `--count`. Same auto-expanding column
/// shape as CSV but tab-separated and unquoted. No hardcoded 4-column header
/// assertion here — see the CSV test above for rationale.
#[test]
fn resource_type_list_tsv_with_count() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_types(&main_view_with_counts(), &count_caveat_meta()).unwrap();
    // Snapshot stdout (header + data rows).
    insta::assert_snapshot!("resource_type_list_tsv_with_count_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land here, not on stdout, and must now
    // carry the count_caveat text.
    insta::assert_snapshot!("resource_type_list_tsv_with_count_stderr", buf_to_string(&err_buf));
}

// ── builtins fixture × 4 formats ─────────────────────────────────────────────
//
// Lines is skipped because it omits `is_builtin` — its builtins cell would be
// indistinguishable from the main fixture's lines cell (name+iri only).

/// Prose render of the builtins fixture. Locks:
/// - `, incl. built-ins` in the header.
/// - `(built-in)` trailing marker on every built-in row.
/// - Interleaved sort: `Archive, AudioSegment, letter, LinkObj, Region, VideoSegment`.
/// - No `(built-in)` marker on project rows.
#[test]
fn resource_type_list_builtins_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_types(&builtins_view(), &anon_meta()).unwrap();
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
    // Project rows must NOT have (built-in) marker.
    let archive_line = out.lines().find(|l| l.contains("Archive")).unwrap();
    assert!(
        !archive_line.contains("(built-in)"),
        "project row must not have (built-in) marker; got: {archive_line:?}"
    );
    // Check interleave: Archive < AudioSegment < letter < LinkObj < Region < VideoSegment.
    let lines: Vec<&str> = out.lines().collect();
    let archive_pos = lines.iter().position(|l| l.contains("Archive")).unwrap();
    let audio_pos = lines.iter().position(|l| l.contains("AudioSegment")).unwrap();
    let letter_pos = lines.iter().position(|l| l.contains("letter")).unwrap();
    let linkobj_pos = lines.iter().position(|l| l.contains("LinkObj")).unwrap();
    let region_pos = lines.iter().position(|l| l.contains("Region")).unwrap();
    let video_pos = lines.iter().position(|l| l.contains("VideoSegment")).unwrap();
    assert!(archive_pos < audio_pos, "Archive must appear before AudioSegment");
    assert!(audio_pos < letter_pos, "AudioSegment must appear before letter");
    assert!(letter_pos < linkobj_pos, "letter must appear before LinkObj");
    assert!(linkobj_pos < region_pos, "LinkObj must appear before Region");
    assert!(region_pos < video_pos, "Region must appear before VideoSegment");
    insta::assert_snapshot!(out);
}

/// JSON render of the builtins fixture. Locks `is_builtin: true` as a JSON boolean
/// for the platform builtins, `label: null` for no-label items (none in this
/// fixture — all builtins have labels), `is_builtin: false` for the project items.
/// Verifies alphabetical interleave in the `data` array.
#[test]
fn resource_type_list_builtins_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_types(&builtins_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    // Structural assertion: is_builtin boolean, interleave order.
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("builtins json must be valid JSON");
    let data = parsed["data"].as_array().unwrap();
    // Archive is first (sorted): is_builtin false
    assert_eq!(data[0]["name"], "Archive", "first item must be Archive (sort order)");
    assert_eq!(data[0]["is_builtin"], false, "Archive must have is_builtin: false");
    // AudioSegment is second: is_builtin true
    assert_eq!(data[1]["name"], "AudioSegment", "second item must be AudioSegment (sort order)");
    assert_eq!(data[1]["is_builtin"], true, "AudioSegment must have is_builtin: true");
    assert_eq!(
        data[1]["label"], "Audio Annotation",
        "AudioSegment must have label 'Audio Annotation'"
    );
    // letter is third: is_builtin false
    assert_eq!(data[2]["name"], "letter", "third item must be letter (sort order)");
    assert_eq!(data[2]["is_builtin"], false, "letter must have is_builtin: false");
    insta::assert_snapshot!(out);
}

/// CSV render of the builtins fixture. Locks `true`/`false` strings in the
/// `is_builtin` column and the interleaved sort visible in row order.
#[test]
fn resource_type_list_builtins_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_types(&builtins_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    // Structural: header present, true/false for is_builtin.
    assert!(
        stdout.starts_with("name,iri,label,is_builtin\n"),
        "CSV header must be name,iri,label,is_builtin; got:\n{stdout}"
    );
    assert!(
        stdout.contains(",true"),
        "CSV builtins rows must contain ',true'; got:\n{stdout}"
    );
    assert!(
        stdout.contains(",false"),
        "CSV builtins rows must contain ',false'; got:\n{stdout}"
    );
    insta::assert_snapshot!("resource_type_list_builtins_csv_stdout", stdout);
    insta::assert_snapshot!("resource_type_list_builtins_csv_stderr", buf_to_string(&err_buf));
}

/// TSV render of the builtins fixture. Same column shape as CSV but tab-separated
/// and unquoted. Locks `true`/`false` in the is_builtin column.
#[test]
fn resource_type_list_builtins_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_types(&builtins_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    assert!(
        stdout.starts_with("name\tiri\tlabel\tis_builtin\n"),
        "TSV header must be name\\tiri\\tlabel\\tis_builtin; got:\n{stdout}"
    );
    assert!(
        stdout.contains("\ttrue"),
        "TSV builtins rows must contain '\\ttrue'; got:\n{stdout}"
    );
    assert!(
        stdout.contains("\tfalse"),
        "TSV builtins rows must contain '\\tfalse'; got:\n{stdout}"
    );
    insta::assert_snapshot!("resource_type_list_builtins_tsv_stdout", stdout);
    insta::assert_snapshot!("resource_type_list_builtins_tsv_stderr", buf_to_string(&err_buf));
}

// ── empty fixture × prose + json ─────────────────────────────────────────────

/// Prose render of the empty fixture. Locks the `(0):` header shape (including the
/// data_model name), the dsp-cli/ADR-0007 footer, and zero rows between them.
#[test]
fn resource_type_list_prose_empty() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_types(&empty_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(out.contains("(0)"), "prose empty must show '(0)' count; got:\n{out}");
    // data_model name must still appear even when empty
    assert!(out.contains("beol"), "prose empty must still show data_model name; got:\n{out}");
    assert!(
        out.contains("[anonymous on"),
        "prose empty must still have disclosure footer; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON render of the empty fixture. Locks `"data":[]` with the standard `_meta`
/// block.
#[test]
fn resource_type_list_json_empty() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_types(&empty_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("empty json must be valid JSON");
    assert!(
        parsed["data"].as_array().unwrap().is_empty(),
        "json empty must have empty data array; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

// ── filter fixture × prose ────────────────────────────────────────────────────

/// Prose render with an active filter. The main fixture has 3 items; filtering by
/// "let" yields 1 match (`letter` — its name contains "let"). Total stays at 3.
/// Locks the "(1 of 3 matching \"let\")" count line and no `incl. built-ins`
/// (no builtins in the filtered set).
#[test]
fn resource_type_list_prose_filter() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    let view = filter_view();
    r.resource_types(&view, &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("matching \"let\""),
        "prose filter must contain 'matching \"let\"'; got:\n{out}"
    );
    assert!(out.contains("of 3"), "prose filter must show 'of 3' total; got:\n{out}");
    insta::assert_snapshot!(out);
}

// ── not_found JSON error envelope ─────────────────────────────────────────────

/// Snapshot of the `not_found` JSON error envelope produced by
/// `renderer.diagnostic(Diagnostic::NotFound(…), &meta)` on a `JsonRenderer`.
///
/// This locks the dsp-cli/ADR-0012 JSON envelope shape — the error path for when
/// `run_list_impl` cannot find the named data-model and returns `NotFound`, which
/// then propagates through the action without ever reaching `resource_types`.
#[test]
fn resource_type_list_json_not_found() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    let diag = Diagnostic::NotFound(
        "data-model 'beol' not found on api.dasch.swiss. \
         Run `dsp vre data-model list --server api.dasch.swiss --project 0801` \
         to see available data-models."
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
            .contains("dsp vre data-model list"),
        "error message must include the recovery hint; got: {}",
        parsed["error"]["message"]
    );

    insta::assert_snapshot!(out);
}

// ── stderr/stdout contract assertions (non-snapshot) ─────────────────────────

/// Lines: disclosure lands on stderr, NOT on stdout.
#[test]
fn resource_type_list_lines_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resource_types(&main_view(), &anon_meta()).unwrap();
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
fn resource_type_list_csv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resource_types(&main_view(), &anon_meta()).unwrap();
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
fn resource_type_list_tsv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resource_types(&main_view(), &anon_meta()).unwrap();
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
/// Note: prose does NOT render IRIs (dsp-cli/ADR-0003), so "ontolog" must also not appear.
/// We check prose only; tabular/json explicitly skip the "ontolog" check since
/// those formats render raw IRIs as data values (e.g. "/ontology/" in IRI paths).
#[test]
fn resource_type_list_prose_no_vocabulary_leak() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resource_types(&main_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.to_lowercase().contains("export"),
        "prose must not contain 'export'; got:\n{out}"
    );
    // Prose renders name + label — no IRIs — so "ontolog" must not appear.
    assert!(
        !out.to_lowercase().contains("ontolog"),
        "prose must not contain 'ontolog' (IRIs are not rendered in prose); got:\n{out}"
    );
    // dsp-cli/ADR-0001 banned terms: "class" and "property" must not appear.
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
fn resource_type_list_json_no_export_leak() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resource_types(&main_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.to_lowercase().contains("export"),
        "json must not contain 'export'; got:\n{out}"
    );
}
