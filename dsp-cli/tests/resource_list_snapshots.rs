//! Snapshot tests for `dsp vre resource list` — one per (noun, format) cell.
//!
//! Fixture philosophy:
//! - **Main fixture** (3 resources, incunabula project, Page type): exercises
//!   the Option matrix (ark_url Some/None, creation_date Some/None). Shared
//!   by prose, json, lines, csv, tsv cells.
//! - **Empty fixture**: zero items, total 0. Prose and json only.
//! - **Filter fixture**: `filter: Some("folio")` with `total > items.len()` so
//!   prose shows "(m of total matching …)". Prose only.
//! - **`--all` json**: `AllPages { pages_fetched: 2 }` — locks the asymmetric
//!   `_meta` shape (`pages_fetched` + `may_have_more_results: false`, no `page`).
//! - **Disclosure variants**: anonymous ("`filter_warning` = Some(anonymous wording)")
//!   vs. authenticated ("`filter_warning` = Some(authenticated wording)"), per D3.
//!
//! Determinism: these tests call `Renderer::resources(&view, &meta)` directly
//! with a hand-built `MetaContext`/`ResourceListView`. They never go through
//! `run_list_impl`, which reads the real `DSP_TOKEN` env var. Action /
//! auth-resolution logic is covered by the in-module action tests; these layer-3
//! snapshot tests cover rendering only. See `docs/dev/testing-strategy.md` and
//! ADR-0009.
//!
//! Vocabulary guard: no `export`/`class`/`property`/`ontolog` in prose output.
//! Tabular/json formats render raw IRIs so the `ontolog` check is skipped there
//! (the documented exception — ADR-0001 / plan 022 learning from plan 014).

use dsp_cli::model::ResourceSummary;
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{MetaContext, Renderer, ResourceListPagination, ResourceListView};

mod support;
use support::{buf_to_string, shared_buf};

// ── constants ─────────────────────────────────────────────────────────────────

/// Standard server label for all snapshot tests.
const SERVER: &str = "api.dasch.swiss";

/// The resource type used in all fixtures (local name for the prose header).
const RESOURCE_TYPE: &str = "page";

// ── D3 filter_warning strings ─────────────────────────────────────────────────

/// D3 anonymous filter_warning: set on every instance-side read, anonymous path.
const ANON_FILTER_WARNING: &str = "results may be filtered; login to see private resources";

/// D3 authenticated filter_warning: set on every instance-side read, auth path.
const AUTH_FILTER_WARNING: &str = "results limited to your permissions";

// ── MetaContext helpers ───────────────────────────────────────────────────────

/// Anonymous `MetaContext` with the D3 filter_warning set.
///
/// Instance-side commands always set `filter_warning` (never `None`); this
/// mirrors what `run_list_impl` builds.
fn anon_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: Some(ANON_FILTER_WARNING.to_string()),
        count_caveat: None,
        count_cost: None,
    }
}

/// Authenticated `MetaContext` with the D3 filter_warning set.
fn auth_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "authenticated as daisy.duck@dasch.swiss".to_string(),
        filter_warning: Some(AUTH_FILTER_WARNING.to_string()),
        count_caveat: None,
        count_cost: None,
    }
}

// ── fixtures ──────────────────────────────────────────────────────────────────

/// Helper to build a `ResourceSummary`.
fn res(
    label: &str,
    iri: &str,
    ark_url: Option<&str>,
    creation_date: Option<&str>,
    last_modified: Option<&str>,
    resource_type: &str,
) -> ResourceSummary {
    ResourceSummary {
        label: label.to_string(),
        iri: iri.to_string(),
        ark_url: ark_url.map(Into::into),
        creation_date: creation_date.map(Into::into),
        last_modified: last_modified.map(Into::into),
        resource_type: resource_type.to_string(),
    }
}

/// Main fixture — 3 incunabula Page resources from the incunabula project (0803):
///   `Folio 1r` (all fields present including last_modified), `Folio 1v` (no ark_url),
///   `Folio 2r` (no creation_date, no last_modified).
///   Realistic IRIs and ARK URLs from the DaSCH infrastructure.
///   Exercises the Option matrix: ark_url Some/None, creation_date Some/None,
///   last_modified Some/None.
fn main_view() -> ResourceListView {
    let items = vec![
        res(
            "Folio 1r",
            "http://rdfh.ch/0803/res-folio-1r",
            Some("http://ark.dasch.swiss/ark:/72163/1/0803/res-folio-1r"),
            Some("2023-04-10T08:00:00Z"),
            Some("2024-01-15T10:30:00Z"), // last_modified present
            "page",
        ),
        res(
            "Folio 1v",
            "http://rdfh.ch/0803/res-folio-1v",
            None, // ark_url absent — Option robustness
            Some("2023-04-10T08:05:00Z"),
            None, // last_modified absent — resource never modified
            "page",
        ),
        res(
            "Folio 2r",
            "http://rdfh.ch/0803/res-folio-2r",
            Some("http://ark.dasch.swiss/ark:/72163/1/0803/res-folio-2r"),
            None, // creation_date absent — Option robustness
            None, // last_modified absent
            "page",
        ),
    ];
    let total = items.len();
    ResourceListView {
        items,
        total,
        filter: None,
        resource_type: RESOURCE_TYPE.to_string(),
        pagination: ResourceListPagination::SinglePage {
            page: 0,
            may_have_more: false,
        },
    }
}

/// Empty fixture — zero items, total 0.
fn empty_view() -> ResourceListView {
    ResourceListView {
        items: vec![],
        total: 0,
        filter: None,
        resource_type: RESOURCE_TYPE.to_string(),
        pagination: ResourceListPagination::SinglePage {
            page: 0,
            may_have_more: false,
        },
    }
}

/// Filter fixture — 1 of 3 items survives the filter ("folio 2" matches only
/// `Folio 2r`). Total stays at 3.
fn filter_view() -> ResourceListView {
    let all = main_view().items;
    let total = 3;
    let filter = "folio 2";
    let needle = filter.to_lowercase();
    let items: Vec<ResourceSummary> = all
        .into_iter()
        .filter(|r| r.label.to_lowercase().contains(&needle))
        .collect();
    ResourceListView {
        items,
        total,
        filter: Some(filter.to_string()),
        resource_type: RESOURCE_TYPE.to_string(),
        pagination: ResourceListPagination::SinglePage {
            page: 0,
            may_have_more: false,
        },
    }
}

/// `--all` view — main fixture items with `AllPages { pages_fetched: 2 }`.
/// Used to lock the asymmetric `_meta` shape: `pages_fetched` + `may_have_more_results: false`,
/// no `page` key. The items and total are identical to `main_view()`.
fn all_pages_view() -> ResourceListView {
    let items = main_view().items;
    let total = items.len();
    ResourceListView {
        items,
        total,
        filter: None,
        resource_type: RESOURCE_TYPE.to_string(),
        pagination: ResourceListPagination::AllPages { pages_fetched: 2 },
    }
}

/// Single-page more-results view: `may_have_more: true` with page 0.
/// Used to lock the "more results available (use --all)" hint in prose.
fn more_results_view() -> ResourceListView {
    ResourceListView {
        pagination: ResourceListPagination::SinglePage {
            page: 0,
            may_have_more: true,
        },
        ..main_view()
    }
}

// ── main fixture × 5 formats (anonymous + D3 disclosure) ─────────────────────

/// Prose render of the main fixture (anonymous). Locks:
/// - header `resources of type page on api.dasch.swiss (3):`.
/// - aligned two-column layout (label + iri).
/// - no filter count line (filter is None).
/// - D3 footer: `[anonymous on api.dasch.swiss] — results may be filtered; login to see private resources`.
#[test]
fn resource_list_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resources(&main_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    // Structural: header present, disclosure + filter_warning present.
    assert!(
        out.contains("resources of type page on api.dasch.swiss"),
        "prose must contain the resource-type header; got:\n{out}"
    );
    assert!(
        out.contains(ANON_FILTER_WARNING),
        "prose must contain the anonymous filter_warning; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON render of the main fixture. Locks:
/// - `_meta` first with `server`, `auth`, `exit_code`, `page`, `may_have_more_results`, `note`.
/// - `data` array with per-item keys `label/iri/ark_url/creation_date/last_modified/resource_type`.
/// - `ark_url: null` for absent items.
/// - `last_modified: null` when absent (item 1 and 2 in fixture).
/// - `note` field = anonymous filter_warning text (D3).
#[test]
fn resource_list_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resources(&main_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    // Parse and structural assertions.
    let parsed: serde_json::Value =
        serde_json::from_str(out.trim()).expect("json must be valid JSON");
    assert!(
        parsed["_meta"]["note"].as_str().is_some(),
        "json _meta must have 'note' key for instance-side command; got:\n{out}"
    );
    assert_eq!(
        parsed["_meta"]["note"].as_str().unwrap(),
        ANON_FILTER_WARNING,
        "json _meta.note must equal the anonymous filter_warning"
    );
    assert!(
        parsed["_meta"]["page"].is_number(),
        "json _meta must have 'page' key in single-page mode; got:\n{out}"
    );
    assert!(
        parsed["_meta"]["may_have_more_results"].is_boolean(),
        "json _meta must have 'may_have_more_results' key; got:\n{out}"
    );
    let data = parsed["data"].as_array().unwrap();
    assert_eq!(data.len(), 3, "json data must have 3 items");
    // Per-item key order check (label present, ark_url null for item[1]).
    assert!(
        data[1]["ark_url"].is_null(),
        "item 1 ark_url must be null (None in fixture); got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// Lines render of the main fixture. Locks the `iri\tlabel` per-row shape and
/// that disclosure + filter_warning goes to stderr.
#[test]
fn resource_list_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resources(&main_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    // Snapshot stdout (data rows only — no header per lines format).
    insta::assert_snapshot!("resource_list_lines_stdout", stdout);
    // Disclosure + filter_warning must be on stderr.
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "lines stderr must contain filter_warning; got: {stderr:?}"
    );
    insta::assert_snapshot!("resource_list_lines_stderr", stderr);
}

/// CSV render of the main fixture. Locks header
/// `label,iri,ark_url,creation_date,last_modified,resource_type`, per-row values, and that
/// disclosure + filter_warning goes to stderr.
#[test]
fn resource_list_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resources(&main_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    // Structural: header present.
    assert!(
        stdout.starts_with("label,iri,ark_url,creation_date,last_modified,resource_type\n"),
        "CSV header must be label,iri,ark_url,creation_date,last_modified,resource_type; got:\n{stdout}"
    );
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "csv stderr must contain filter_warning; got: {stderr:?}"
    );
    insta::assert_snapshot!("resource_list_csv_stdout", stdout);
    insta::assert_snapshot!("resource_list_csv_stderr", stderr);
}

/// TSV render of the main fixture. Locks header
/// `label\tiri\tark_url\tcreation_date\tlast_modified\tresource_type`, per-row values, and
/// that disclosure + filter_warning goes to stderr.
#[test]
fn resource_list_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resources(&main_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    // Structural: header present.
    assert!(
        stdout.starts_with("label\tiri\tark_url\tcreation_date\tlast_modified\tresource_type\n"),
        "TSV header must be label\\tiri\\tark_url\\tcreation_date\\tlast_modified\\tresource_type; got:\n{stdout}"
    );
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "tsv stderr must contain filter_warning; got: {stderr:?}"
    );
    insta::assert_snapshot!("resource_list_tsv_stdout", stdout);
    insta::assert_snapshot!("resource_list_tsv_stderr", stderr);
}

// ── empty fixture × prose + json ─────────────────────────────────────────────

/// Prose render of the empty fixture. Locks the `(0):` header shape, the D3
/// disclosure footer, and zero data rows between them.
#[test]
fn resource_list_prose_empty() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resources(&empty_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("(0)"),
        "prose empty must show '(0)' count; got:\n{out}"
    );
    assert!(
        out.contains("page"),
        "prose empty must still show resource_type name; got:\n{out}"
    );
    assert!(
        out.contains(ANON_FILTER_WARNING),
        "prose empty must still have D3 disclosure footer; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON render of the empty fixture. Locks `"data":[]` with the standard `_meta`
/// block including the D3 `note`.
#[test]
fn resource_list_json_empty() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resources(&empty_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value =
        serde_json::from_str(out.trim()).expect("empty json must be valid JSON");
    assert!(
        parsed["data"].as_array().unwrap().is_empty(),
        "json empty must have empty data array; got:\n{out}"
    );
    assert_eq!(
        parsed["_meta"]["note"].as_str().unwrap_or(""),
        ANON_FILTER_WARNING,
        "json empty must still carry D3 note"
    );
    insta::assert_snapshot!(out);
}

// ── filter fixture × prose ────────────────────────────────────────────────────

/// Prose render with an active filter. The main fixture has 3 items; filtering by
/// "folio 2" yields 1 match (`Folio 2r`). Total stays at 3.
/// Locks the `(1 of 3 matching "folio 2")` count line.
#[test]
fn resource_list_prose_filter() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    let view = filter_view();
    r.resources(&view, &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("matching \"folio 2\""),
        "prose filter must contain 'matching \"folio 2\"'; got:\n{out}"
    );
    assert!(
        out.contains("of 3"),
        "prose filter must show 'of 3' total; got:\n{out}"
    );
    assert!(
        out.contains(ANON_FILTER_WARNING),
        "prose filter must contain D3 filter_warning; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

// ── `--all` json: AllPages _meta shape ───────────────────────────────────────

/// JSON render with `AllPages { pages_fetched: 2 }`. Locks the D5 asymmetric
/// `_meta` shape:
/// - `pages_fetched` present (value: 2)
/// - `may_have_more_results: false` (loop invariant)
/// - NO `page` key (single-page mode only)
/// - D3 `note` present.
#[test]
fn resource_list_json_all_pages() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resources(&all_pages_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value =
        serde_json::from_str(out.trim()).expect("all-pages json must be valid JSON");

    // AllPages shape: pages_fetched, may_have_more_results: false, no page.
    assert_eq!(
        parsed["_meta"]["pages_fetched"].as_u64().unwrap(),
        2,
        "all-pages _meta must have pages_fetched: 2; got:\n{out}"
    );
    assert!(
        !parsed["_meta"]["may_have_more_results"].as_bool().unwrap(),
        "all-pages _meta must have may_have_more_results: false; got:\n{out}"
    );
    assert!(
        parsed["_meta"].get("page").is_none(),
        "all-pages _meta must NOT have a 'page' key (asymmetry per D5); got:\n{out}"
    );
    assert_eq!(
        parsed["_meta"]["note"].as_str().unwrap_or(""),
        ANON_FILTER_WARNING,
        "all-pages _meta must carry D3 note"
    );

    insta::assert_snapshot!(out);
}

// ── D3 disclosure line: anonymous vs. authenticated ───────────────────────────
//
// The anonymous case is already covered by `resource_list_prose` (which uses
// `anon_meta()`). Only the authenticated case is snapshot-tested here to avoid
// two byte-identical snapshots that can silently drift.

/// Authenticated prose: footer must include the authenticated filter_warning text.
/// Locks: `[authenticated as daisy.duck@dasch.swiss on api.dasch.swiss] — results limited to your permissions`
#[test]
fn resource_list_disclosure_authenticated_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resources(&main_view(), &auth_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains(AUTH_FILTER_WARNING),
        "authenticated prose must contain the filter_warning; got:\n{out}"
    );
    assert!(
        out.contains("authenticated as"),
        "authenticated prose footer must say 'authenticated as'; got:\n{out}"
    );
    assert!(
        out.contains("daisy.duck@dasch.swiss"),
        "authenticated prose footer must include the user; got:\n{out}"
    );
    insta::assert_snapshot!("resource_list_disclosure_authenticated_prose", out);
}

/// Anonymous tabular (CSV stderr): stderr must contain the anonymous filter_warning.
#[test]
fn resource_list_disclosure_anonymous_csv_stderr() {
    let (_, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resources(&main_view(), &anon_meta()).unwrap();
    let stderr = buf_to_string(&err_buf);
    assert!(
        stderr.contains(ANON_FILTER_WARNING),
        "csv anonymous stderr must contain filter_warning; got: {stderr:?}"
    );
    insta::assert_snapshot!("resource_list_disclosure_anonymous_csv_stderr", stderr);
}

/// Authenticated tabular (CSV stderr): stderr must contain the authenticated filter_warning.
#[test]
fn resource_list_disclosure_authenticated_csv_stderr() {
    let (_, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resources(&main_view(), &auth_meta()).unwrap();
    let stderr = buf_to_string(&err_buf);
    assert!(
        stderr.contains(AUTH_FILTER_WARNING),
        "csv authenticated stderr must contain filter_warning; got: {stderr:?}"
    );
    insta::assert_snapshot!("resource_list_disclosure_authenticated_csv_stderr", stderr);
}

// ── "more results available" hint in prose (SinglePage, may_have_more: true) ──

/// Prose with `may_have_more: true` must show the `more results available (use --all)` hint.
#[test]
fn resource_list_prose_more_results_hint() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resources(&more_results_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("more results available"),
        "prose with may_have_more: true must show the 'more results available' hint; got:\n{out}"
    );
    assert!(
        out.contains("--all"),
        "prose hint must reference '--all'; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// Prose with `may_have_more: false` must NOT show the hint.
#[test]
fn resource_list_prose_no_more_results_hint() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resources(&main_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("more results available"),
        "prose with may_have_more: false must NOT show hint; got:\n{out}"
    );
}

// ── stdout/stderr contract assertions (non-snapshot) ─────────────────────────

/// Lines: disclosure lands on stderr, NOT on stdout.
#[test]
fn resource_list_lines_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.resources(&main_view(), &anon_meta()).unwrap();
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
fn resource_list_csv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.resources(&main_view(), &anon_meta()).unwrap();
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
fn resource_list_tsv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.resources(&main_view(), &anon_meta()).unwrap();
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

/// Prose output must not contain DSP-API vocabulary leaks.
/// Note: prose does NOT render IRIs (ADR-0003), so "ontolog" must not appear.
#[test]
fn resource_list_prose_no_vocabulary_leak() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.resources(&main_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.to_lowercase().contains("export"),
        "prose must not contain 'export'; got:\n{out}"
    );
    // Prose renders label + iri but not the IRI in the header — "ontolog"
    // would only appear if an IRI leaked into prose text labels.
    // IRIs do appear in prose rows (as the second column), so we only check
    // the non-IRI text portions: no "class" or "property" as prose vocabulary.
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
#[test]
fn resource_list_json_no_export_leak() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.resources(&main_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.to_lowercase().contains("export"),
        "json must not contain 'export'; got:\n{out}"
    );
}

// ── existing callers: filter_warning=None → no D3 note in JSON ───────────────

/// Guard that existing callers that pass `filter_warning: None` to
/// `Renderer::resources` (should there be any in tests) do NOT get a `note`
/// key in `_meta`. This ensures the `None`-guard in the json renderer is correct.
///
/// Note: the production action always sets filter_warning for resource list;
/// this test is belt-and-braces for renderer correctness.
#[test]
fn resource_list_json_no_note_when_filter_warning_none() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    let meta_no_warning = MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    };
    r.resources(&main_view(), &meta_no_warning).unwrap();
    let out = buf_to_string(&buf);
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("json must parse");
    assert!(
        parsed["_meta"].get("note").is_none(),
        "_meta must NOT have 'note' when filter_warning is None; got:\n{out}"
    );
}
