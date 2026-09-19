//! Snapshot tests for `dsp vre project list` — one per (noun, format) cell.
//!
//! Fixture philosophy: the five format cells (and the authenticated / empty /
//! iri / vocabulary tests) all share ONE *realistic* fixture
//! (`realistic_projects`) — plausible DaSCH-style projects with no special
//! characters, so each format snapshot reads like real output. Edge cases that
//! need pathological input are kept in their OWN dedicated fixtures so they
//! never bleed into the shared snapshots:
//!
//! - CSV/TSV quoting (comma, leading `=`, embedded quote) → `csv_escaping_view`
//! - non-ASCII filter echo → the `project_list_prose_filter_non_ascii` test
//!
//! This keeps test cases from influencing each other through a shared fixture.
//!
//! Determinism: these tests call `Renderer::projects(&view, &meta)` directly
//! with a hand-built `MetaContext`/`ProjectListView` (the action `run_list_impl`
//! is private and reads `DSP_TOKEN` from the real env). The action path (sort,
//! filter, auth-state derivation, cache fallback) is covered by the in-module
//! action tests and the wiremock tests; these layer-3 snapshots cover rendering
//! only. See `docs/src/dsp-cli/testing-strategy.md`.
//!
//! Realistic fixture (`realistic_projects`), returned in server order (unsorted;
//! the renderer receives the already-sorted view, so snapshots show ascending
//! shortcode order `0512 / 0801 / 0820 / 0918`):
//! - `0801` / `beol` / Active / 4 data-models / "Bernoulli-Euler Online"
//! - `0918` / `roud` / Active / 2 data-models / "Gustave Roud"
//! - `0512` / `sandbox` / Inactive / 0 data-models / longname `None` (legitimate field states:
//!   inactive, zero data-models, absent longname)
//! - `0820` / `incunabula` / Active / 1 data-model / "Basel Early Book Printing"
//!
//! dsp-cli/ADR-0001 vocabulary guard: no `ontology`/`export`/`class`/`property` in this
//! file or any .snap it generates.

use dsp_cli::model::{Project, ProjectStatus};
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{MetaContext, ProjectListView, Renderer};

mod support;
use support::{buf_to_string, shared_buf};

fn project(
    iri: &str,
    shortcode: &str,
    shortname: &str,
    longname: Option<&str>,
    status: ProjectStatus,
    data_models: usize,
) -> Project {
    Project {
        iri: iri.into(),
        shortcode: shortcode.into(),
        shortname: shortname.into(),
        longname: longname.map(Into::into),
        status,
        data_models,
    }
}

// ── realistic fixture (shared by the format cells) ───────────────────────────

/// Realistic, special-character-free fixture. Returned in server order
/// (unsorted) so the sort in `full_view` is meaningful. Covers the legitimate
/// field-state variations (active/inactive, multi/single/zero data-models, an
/// absent longname) without any quoting/escaping bait.
fn realistic_projects() -> Vec<Project> {
    use ProjectStatus::{Active, Inactive};
    vec![
        project(
            "http://rdfh.ch/projects/Qt8K2mWbT0eHa1cZ",
            "0801",
            "beol",
            Some("Bernoulli-Euler Online"),
            Active,
            4,
        ),
        project(
            "http://rdfh.ch/projects/Lp3R9vNxS7iWd4Bg",
            "0918",
            "roud",
            Some("Gustave Roud"),
            Active,
            2,
        ),
        project("http://rdfh.ch/projects/Zc6F1hYpQ2kMe8Vn", "0512", "sandbox", None, Inactive, 0),
        project(
            "http://rdfh.ch/projects/Hn5D0sJwR3uXf7Tb",
            "0820",
            "incunabula",
            Some("Basel Early Book Printing"),
            Active,
            1,
        ),
    ]
}

/// Standard server for all snapshot tests.
const SERVER: &str = "https://api.test.dasch.swiss";

// ── helpers ───────────────────────────────────────────────────────────────────

/// Fully synthetic `MetaContext` for anonymous snapshots.
fn anon_meta() -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: "anonymous".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

/// Authenticated `MetaContext` (cache token with user).
fn authed_meta(user: &str) -> MetaContext {
    MetaContext {
        server_label: SERVER.to_string(),
        auth_state: format!("authenticated as {user}"),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

/// Sort projects ascending by shortcode — mirrors what `run_list_impl` does
/// before handing the view to the renderer.
fn sorted(mut items: Vec<Project>) -> Vec<Project> {
    items.sort_by(|a, b| a.shortcode.cmp(&b.shortcode));
    items
}

/// Apply a case-insensitive substring filter over shortcode/shortname/longname,
/// mirroring `run_list_impl`. Returns `(matching, total)`.
fn apply_filter(all: Vec<Project>, filter: &str) -> (Vec<Project>, usize) {
    let total = all.len();
    let needle = filter.to_lowercase();
    let items: Vec<Project> = all
        .into_iter()
        .filter(|p| {
            p.shortcode.to_lowercase().contains(&needle)
                || p.shortname.to_lowercase().contains(&needle)
                || p.longname.as_deref().unwrap_or("").to_lowercase().contains(&needle)
        })
        .collect();
    (sorted(items), total)
}

/// The realistic fixture as a no-filter view.
fn full_view() -> ProjectListView {
    let items = sorted(realistic_projects());
    let total = items.len();
    ProjectListView { items, total, filter: None }
}

/// Realistic fixture filtered by `"online"` — matches only `beol`
/// ("Bernoulli-Euler Online"), so the count line reads "1 of 4 matching".
fn filtered_view() -> ProjectListView {
    let (items, total) = apply_filter(realistic_projects(), "online");
    ProjectListView { items, total, filter: Some("online".to_string()) }
}

/// Realistic fixture with a non-matching filter → empty result.
fn empty_view() -> ProjectListView {
    let (items, total) = apply_filter(realistic_projects(), "no-match-xyzzy");
    ProjectListView { items, total, filter: Some("no-match-xyzzy".to_string()) }
}

// ── cells: realistic fixture × 5 formats (anonymous) ─────────────────────────

#[test]
fn project_list_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.projects(&full_view(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn project_list_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.projects(&full_view(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn project_list_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.projects(&full_view(), &anon_meta()).unwrap();
    // Snapshot stdout (data rows only — no header per lines format).
    insta::assert_snapshot!("project_list_lines_stdout", buf_to_string(&out_buf));
    // Snapshot stderr separately: the disclosure line must land here, not on stdout.
    insta::assert_snapshot!("project_list_lines_stderr", buf_to_string(&err_buf));
}

#[test]
fn project_list_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.projects(&full_view(), &anon_meta()).unwrap();
    // Snapshot stdout (header + data rows).
    insta::assert_snapshot!("project_list_tsv_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land on err, not stdout.
    insta::assert_snapshot!("project_list_tsv_stderr", buf_to_string(&err_buf));
}

#[test]
fn project_list_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.projects(&full_view(), &anon_meta()).unwrap();
    // Snapshot stdout (header + data rows). Realistic data needs no quoting —
    // quoting/escaping is exercised by `project_list_csv_escaping` below.
    insta::assert_snapshot!("project_list_csv_stdout", buf_to_string(&out_buf));
    // Snapshot stderr: disclosure must land on err, not stdout.
    insta::assert_snapshot!("project_list_csv_stderr", buf_to_string(&err_buf));
}

// ── filter prose snapshot: "m of n matching" count line ──────────────────────

/// Prose with `--filter "online"`: one match out of four. Locks the count-line
/// shape with realistic data (no special characters).
#[test]
fn project_list_prose_filtered() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.projects(&filtered_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("1 of 4 matching \"online\""),
        "prose filtered must contain '1 of 4 matching \"online\"'; got: {out}"
    );
    insta::assert_snapshot!(out);
}

// ── empty-result case (non-matching filter) ───────────────────────────────────

/// Prose with non-matching filter: must show "(0 of N matching …)".
#[test]
fn project_list_prose_empty() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.projects(&empty_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("0 of 4 matching"),
        "prose empty must contain '0 of 4 matching'; got: {out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON with non-matching filter: `"data": []`.
#[test]
fn project_list_json_empty() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.projects(&empty_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(out.contains("\"data\":[]"), "json empty must contain '\"data\":[]'; got: {out}");
    insta::assert_snapshot!(out);
}

/// Lines with non-matching filter: only the stderr disclosure, no data rows.
#[test]
fn project_list_lines_empty() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.projects(&empty_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    // No data rows on stdout.
    assert!(stdout.is_empty(), "lines empty must have no stdout data rows; got: {stdout:?}");
    // Disclosure still on stderr.
    assert!(
        stderr.contains("[anonymous on"),
        "lines empty must still have disclosure on stderr; got: {stderr:?}"
    );
    insta::assert_snapshot!("project_list_lines_empty_stdout", stdout);
    insta::assert_snapshot!("project_list_lines_empty_stderr", stderr);
}

/// TSV with non-matching filter: header row only on stdout.
#[test]
fn project_list_tsv_empty() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.projects(&empty_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    // Only the header row on stdout.
    assert!(
        stdout.starts_with("shortcode\t"),
        "tsv empty must start with header row; got: {stdout:?}"
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "tsv empty must have exactly one line (header only); got: {stdout:?}"
    );
    insta::assert_snapshot!("project_list_tsv_empty_stdout", stdout);
    insta::assert_snapshot!("project_list_tsv_empty_stderr", stderr);
}

/// CSV with non-matching filter: header row only on stdout.
#[test]
fn project_list_csv_empty() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.projects(&empty_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let stderr = buf_to_string(&err_buf);
    // Only the header row on stdout.
    assert!(
        stdout.starts_with("shortcode,"),
        "csv empty must start with header row; got: {stdout:?}"
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "csv empty must have exactly one line (header only); got: {stdout:?}"
    );
    insta::assert_snapshot!("project_list_csv_empty_stdout", stdout);
    insta::assert_snapshot!("project_list_csv_empty_stderr", stderr);
}

// ── authenticated-footer variant ──────────────────────────────────────────────

/// Prose with authenticated user in `MetaContext`: footer must show
/// "[authenticated as alice@example.com on <server>]".
#[test]
fn project_list_prose_authenticated() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.projects(&full_view(), &authed_meta("alice@example.com")).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("authenticated as alice@example.com"),
        "prose authed must show 'authenticated as alice@example.com'; got: {out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON with authenticated user: `_meta.auth` must show "authenticated as <user>".
#[test]
fn project_list_json_authenticated() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.projects(&full_view(), &authed_meta("alice@example.com")).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("authenticated as alice@example.com"),
        "json authed must show 'authenticated as alice@example.com' in _meta.auth; got: {out}"
    );
    insta::assert_snapshot!(out);
}

// ── dedicated edge-case fixtures (isolated from the realistic fixture) ─────────

/// Adversarial fixture used ONLY by the CSV/TSV quoting tests: longnames that
/// trigger every quoting path — an embedded comma, a leading `=` (spreadsheet
/// formula-injection), and an embedded double-quote (RFC-4180 doubling). Kept
/// out of the shared realistic fixture so it never pollutes the other cells.
fn csv_escaping_view() -> ProjectListView {
    use ProjectStatus::Active;
    let items = sorted(vec![
        project(
            "http://rdfh.ch/projects/Ed1tNsXqT0",
            "0001",
            "editions",
            Some("Letters, Drafts and Notes"),
            Active,
            2,
        ),
        project(
            "http://rdfh.ch/projects/Ca1cFmLpR2",
            "0002",
            "ledger",
            Some("=SUM(revenue)"),
            Active,
            1,
        ),
        project(
            "http://rdfh.ch/projects/Qu0tEdZwV3",
            "0003",
            "quoted",
            Some("The \"Definitive\" Edition"),
            Active,
            1,
        ),
    ]);
    let total = items.len();
    ProjectListView { items, total, filter: None }
}

/// CSV quoting: comma → wrapped; leading `=` → wrapped (formula-injection
/// mitigation); embedded `"` → wrapped with the quote doubled. Asserted
/// explicitly *and* snapshotted so the quoting contract is locked in one place.
#[test]
fn project_list_csv_escaping() {
    let (out_buf, out_w) = shared_buf();
    let (_, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.projects(&csv_escaping_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&out_buf);
    assert!(
        out.contains("\"Letters, Drafts and Notes\""),
        "csv must quote a comma-bearing field; got: {out}"
    );
    assert!(
        out.contains("\"=SUM(revenue)\""),
        "csv must quote a leading-'=' field (formula-injection mitigation); got: {out}"
    );
    assert!(
        out.contains("\"The \"\"Definitive\"\" Edition\""),
        "csv must double embedded quotes (RFC-4180); got: {out}"
    );
    insta::assert_snapshot!("project_list_csv_escaping_stdout", out);
}

/// TSV does NOT quote (per the v1 contract — fields are assumed tab-free); the
/// same adversarial longnames pass through verbatim. Locks that TSV and CSV
/// diverge on quoting.
#[test]
fn project_list_tsv_no_quoting() {
    let (out_buf, out_w) = shared_buf();
    let (_, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.projects(&csv_escaping_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&out_buf);
    assert!(
        out.contains("=SUM(revenue)") && !out.contains("\"=SUM(revenue)\""),
        "tsv must pass the leading-'=' field through unquoted; got: {out}"
    );
    insta::assert_snapshot!("project_list_tsv_escaping_stdout", out);
}

/// Non-ASCII filter echo: the `--filter` value is opaque display text, echoed
/// verbatim in the prose count line (not sanitised). Isolated to its own
/// fixture/filter so the non-ASCII byte never appears in the other snapshots.
#[test]
fn project_list_prose_filter_non_ascii() {
    use ProjectStatus::Active;
    let all = vec![project(
        "http://rdfh.ch/projects/Ca1fEzWqT0",
        "0007",
        "cafe",
        Some("Café Editions"),
        Active,
        1,
    )];
    let (items, total) = apply_filter(all, "café");
    let view = ProjectListView { items, total, filter: Some("café".to_string()) };

    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.projects(&view, &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("matching \"café\""),
        "prose must echo the non-ASCII filter value 'café' verbatim; got: {out}"
    );
    insta::assert_snapshot!("project_list_prose_filter_non_ascii", out);
}

// ── stdout/stderr contract assertions (non-snapshot) ─────────────────────────

/// Lines: disclosure lands on stderr, NOT on stdout.
#[test]
fn project_list_lines_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.projects(&full_view(), &anon_meta()).unwrap();
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

/// TSV: disclosure lands on stderr, NOT on stdout.
#[test]
fn project_list_tsv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.projects(&full_view(), &anon_meta()).unwrap();
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

/// CSV: disclosure lands on stderr, NOT on stdout.
#[test]
fn project_list_csv_disclosure_on_stderr_not_stdout() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.projects(&full_view(), &anon_meta()).unwrap();
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

// ── vocabulary guard (inline) ─────────────────────────────────────────────────

/// Prose and JSON output must not contain vocabulary-leaked words from the
/// DSP-API layer: "ontology", "export", "class" (as field), "property".
/// This is a belt-and-braces check; the snapshots are the canonical contract.
#[test]
fn project_list_prose_no_vocabulary_leak() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.projects(&full_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.to_lowercase().contains("ontolog"),
        "prose must not contain 'ontolog'; got: {out}"
    );
    assert!(
        !out.to_lowercase().contains("export"),
        "prose must not contain 'export'; got: {out}"
    );
}

#[test]
fn project_list_json_no_vocabulary_leak() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.projects(&full_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.to_lowercase().contains("ontolog"),
        "json must not contain 'ontolog'; got: {out}"
    );
    assert!(
        !out.to_lowercase().contains("export"),
        "json must not contain 'export'; got: {out}"
    );
}

// ── iri absent in prose, present in tsv/csv/json ──────────────────────────────

/// Prose: `iri` field must NOT appear (per dsp-cli/ADR-0003 / plan Step 3c).
#[test]
fn project_list_prose_no_iri() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.projects(&full_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        !out.contains("http://rdfh.ch/projects/"),
        "prose must not contain IRI; got: {out}"
    );
}

/// TSV: `iri` column must be present in the header and all data rows.
#[test]
fn project_list_tsv_has_iri() {
    let (out_buf, out_w) = shared_buf();
    let (_, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.projects(&full_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&out_buf);
    assert!(out.contains("iri"), "tsv must have iri column in header; got: {out}");
    assert!(
        out.contains("http://rdfh.ch/projects/"),
        "tsv must have iri values in data rows; got: {out}"
    );
}

/// CSV: `iri` column must be present in the header and all data rows.
#[test]
fn project_list_csv_has_iri() {
    let (out_buf, out_w) = shared_buf();
    let (_, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.projects(&full_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&out_buf);
    assert!(out.contains("iri"), "csv must have iri column in header; got: {out}");
    assert!(
        out.contains("http://rdfh.ch/projects/"),
        "csv must have iri values in data rows; got: {out}"
    );
}

/// JSON: `iri` key must be present in every data object.
#[test]
fn project_list_json_has_iri() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.projects(&full_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(out.contains("\"iri\""), "json must have iri key; got: {out}");
    assert!(
        out.contains("http://rdfh.ch/projects/"),
        "json must have iri values; got: {out}"
    );
}
