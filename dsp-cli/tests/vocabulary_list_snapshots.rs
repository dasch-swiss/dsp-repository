//! Snapshot tests for `dsp vre vocabulary list` — one per (fixture, format) cell.
//!
//! Fixture philosophy:
//! - **Main fixture** (4 geoarch-like vocabularies: `epoch`, `material`, `person`, and one with
//!   `name: None`): pre-sorted by name (case-insensitive, `None`-named last) the way the action
//!   would hand it to the renderer. Exercises the label Option matrix (2 languages + a comment, 1
//!   language + no comment, name absent). `node_count`/`depth` are `None` on every item (no
//!   `--count`). Shared by prose, json, lines, csv, tsv cells.
//! - **Counted fixture** — the main fixture's items with `node_count`/`depth` populated on every
//!   item, `counted: true`, and `meta.count_cost` set. Exercises the `nodes`/`depth` tabular
//!   auto-default and the prose `· N nodes · M levels` per-item suffix.
//! - **All-failed-counted fixture** — same items, but `node_count`/`depth` stay `None` on every
//!   item even though `counted: true` and `count_cost` is `Some` (mirrors a `--count` run where
//!   every per-vocabulary tree fetch failed). Locks that the dynamic tabular default keys off
//!   actual per-item data, NOT the `counted` flag.
//! - **Empty fixture**: zero items, total 0. Prose only.
//! - **Filter fixture**: `filter: Some("epo")` with `total > items.len()` so the prose header shows
//!   "(m of total matching …)". Prose only.
//!
//! Determinism: these tests call `Renderer::vocabularies(&view, &meta)`
//! **directly** with a hand-built `MetaContext` / `VocabularyListView`. They never
//! go through `run_list_impl`, which reads the real `DSP_TOKEN` env var. Action /
//! auth-resolution / sorting logic is covered by the in-module action tests; these
//! layer-4 snapshot tests cover rendering only.
//! See `docs/dev/testing-strategy.md`, ADR-0009, and
//! `docs/design/plans/034-vre-vocabulary/implementation-plan.md`.

use dsp_cli::model::{LocalizedText, Vocabulary, VocabularyHeader};
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{MetaContext, Renderer, VocabularyListView};

mod support;
use support::{buf_to_string, shared_buf};

// ── server constant ───────────────────────────────────────────────────────────

/// Standard server label for all snapshot tests — mirrors what `run_list_impl`
/// sets from `cfg.server`.
const SERVER: &str = "api.dasch.swiss";

/// Copied verbatim from `COUNT_COST` in `src/actions/vre/vocabulary.rs` so the
/// two stay in sync.
const COUNT_COST: &str = "--count issues one extra tree fetch per vocabulary (sequential; can be dozens of calls on projects with many vocabularies).";

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

/// Anonymous `MetaContext` with `count_cost` set — mirrors what `run_list_impl`
/// builds when `--count` is passed and every per-tree fetch succeeded.
fn count_cost_meta() -> MetaContext {
    MetaContext { count_cost: Some(COUNT_COST.to_string()), ..anon_meta() }
}

/// Anonymous `MetaContext` with `count_cost` set AND the failure-tally suffix
/// `run_list_impl` appends when per-vocabulary tree fetches failed — here, all
/// 4 of 4 failed.
fn count_cost_all_failed_meta() -> MetaContext {
    MetaContext {
        count_cost: Some(format!(
            "{COUNT_COST} 4 of 4 per-vocabulary tree fetches failed; affected rows show no node/depth counts."
        )),
        ..anon_meta()
    }
}

// ── fixtures ──────────────────────────────────────────────────────────────────

fn text(value: &str, language: Option<&str>) -> LocalizedText {
    LocalizedText {
        value: value.to_string(),
        language: language.map(str::to_string),
    }
}

fn header(iri: &str, name: Option<&str>, labels: Vec<LocalizedText>, comments: Vec<LocalizedText>) -> VocabularyHeader {
    VocabularyHeader {
        iri: iri.to_string(),
        name: name.map(str::to_string),
        labels,
        comments,
    }
}

fn vocab(header: VocabularyHeader, node_count: Option<usize>, depth: Option<usize>) -> Vocabulary {
    Vocabulary { header, node_count, depth }
}

/// Main fixture — 4 geoarch-like vocabularies, pre-sorted the way the action
/// would (case-insensitive name ascending, `None`-named last):
/// `epoch` (2 languages + a comment), `material` (1 language, no comment),
/// `person` (2 languages + a comment), and one with `name: None`.
fn main_view() -> VocabularyListView {
    let items = vec![
        vocab(
            header(
                "http://rdfh.ch/lists/0838/epoch",
                Some("epoch"),
                vec![text("Period", Some("en")), text("Epoche", Some("de"))],
                vec![text("Chronological periods used for dating find contexts.", Some("en"))],
            ),
            None,
            None,
        ),
        vocab(
            header(
                "http://rdfh.ch/lists/0838/material",
                Some("material"),
                vec![text("Material", Some("en"))],
                vec![],
            ),
            None,
            None,
        ),
        vocab(
            header(
                "http://rdfh.ch/lists/0838/person",
                Some("person"),
                vec![text("Person", Some("en")), text("Person", Some("de"))],
                vec![text(
                    "People associated with the excavation or its publication.",
                    Some("en"),
                )],
            ),
            None,
            None,
        ),
        vocab(
            header(
                "http://rdfh.ch/lists/0838/unnamed01",
                None,
                vec![text("Miscellaneous", Some("en"))],
                vec![],
            ),
            None,
            None,
        ),
    ];
    let total = items.len();
    VocabularyListView { items, total, filter: None, counted: false }
}

/// Counted fixture — the main fixture's items with `node_count`/`depth`
/// populated on every item and `counted: true`.
fn counted_view() -> VocabularyListView {
    let mut view = main_view();
    let counts: [(usize, usize); 4] = [(33, 3), (12, 2), (87, 3), (5, 1)];
    for (item, (n, d)) in view.items.iter_mut().zip(counts) {
        item.node_count = Some(n);
        item.depth = Some(d);
    }
    view.counted = true;
    view
}

/// All-failed-counted fixture — same items as the main fixture, `counted: true`,
/// but `node_count`/`depth` stay `None` on every item (every per-vocabulary tree
/// fetch failed).
fn all_failed_view() -> VocabularyListView {
    let mut view = main_view();
    view.counted = true;
    view
}

/// Empty fixture — zero items, total 0.
fn empty_view() -> VocabularyListView {
    VocabularyListView { items: vec![], total: 0, filter: None, counted: false }
}

/// Filter fixture — 1 of 4 items survives the filter ("epo" matches only
/// `epoch`'s name), so prose shows "(1 of 4 matching \"epo\")".
fn filter_view() -> VocabularyListView {
    let all = main_view().items;
    let total = all.len();
    let filter = "epo";
    let needle = filter.to_lowercase();
    let items: Vec<Vocabulary> = all
        .into_iter()
        .filter(|v| v.header.name.as_deref().unwrap_or("").to_lowercase().contains(&needle))
        .collect();
    VocabularyListView {
        items,
        total,
        filter: Some(filter.to_string()),
        counted: false,
    }
}

// ── main fixture × 5 formats (anonymous) ─────────────────────────────────────

/// Prose render of the main fixture. Locks the header count line, the
/// `name  labels` layout (D13 language-join order), no per-item count suffix
/// (no `--count`), and the ADR-0007 footer.
#[test]
fn vocabulary_list_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.vocabularies(&main_view(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// JSON render of the main fixture. Locks the `_meta`-first envelope, `"data"`
/// as an array, per-item key order (name/iri/labels/comments), lossless
/// `{value, language}` arrays, and `nodes`/`depth` OMITTED (not null) when the
/// item carries no count.
#[test]
fn vocabulary_list_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.vocabularies(&main_view(), &anon_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

/// Lines render of the main fixture. Locks the `name\tiri` per-row shape (no
/// header, no label/comment columns — lean chaining format) and that
/// disclosure goes to stderr.
#[test]
fn vocabulary_list_lines() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = LinesRenderer::with_writers(out_w, err_w);
    r.vocabularies(&main_view(), &anon_meta()).unwrap();
    insta::assert_snapshot!("vocabulary_list_lines_stdout", buf_to_string(&out_buf));
    insta::assert_snapshot!("vocabulary_list_lines_stderr", buf_to_string(&err_buf));
}

/// CSV render of the main fixture. Locks the 8-column plain default header
/// (`name,iri,label_en,label_de,label_fr,label_it,label_rm,label` — no
/// `nodes`/`depth`, since no item carries a count) and that disclosure goes to
/// stderr.
#[test]
fn vocabulary_list_csv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.vocabularies(&main_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    assert!(
        stdout.starts_with("name,iri,label_en,label_de,label_fr,label_it,label_rm,label\n"),
        "CSV header must be the plain 8-column default; got:\n{stdout}"
    );
    insta::assert_snapshot!("vocabulary_list_csv_stdout", stdout);
    insta::assert_snapshot!("vocabulary_list_csv_stderr", buf_to_string(&err_buf));
}

/// TSV render of the main fixture. Same columns as CSV but tab-separated and
/// unquoted.
#[test]
fn vocabulary_list_tsv() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.vocabularies(&main_view(), &anon_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    assert!(
        stdout.starts_with("name\tiri\tlabel_en\tlabel_de\tlabel_fr\tlabel_it\tlabel_rm\tlabel\n"),
        "TSV header must be the plain 8-column default; got:\n{stdout}"
    );
    insta::assert_snapshot!("vocabulary_list_tsv_stdout", stdout);
    insta::assert_snapshot!("vocabulary_list_tsv_stderr", buf_to_string(&err_buf));
}

// ── counted fixture (all items carry counts) ─────────────────────────────────

/// Prose render of the counted fixture. Locks the per-item `· N nodes · M
/// levels` suffix and the `count_cost` disclosure appended to the ADR-0007
/// footer.
#[test]
fn vocabulary_list_prose_with_count() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.vocabularies(&counted_view(), &count_cost_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("33 nodes") && out.contains("3 levels"),
        "prose with-count must show the epoch row's node/level suffix; got:\n{out}"
    );
    assert!(
        out.contains(COUNT_COST),
        "prose with-count footer must carry the count_cost text; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON render of the counted fixture. Locks that `_meta.note` carries the
/// `count_cost` disclosure text (json's own per-method `_meta["note"]`
/// assignment in `Renderer::vocabularies` — a NEW `MetaContext` field is not
/// self-rendering, per the plan's ripple warning, so this specific wiring
/// needs its own test rather than relying on the prose/tabular-stderr
/// coverage above) and that `nodes`/`depth` per-item keys ARE present (every
/// item in this fixture carries a count).
#[test]
fn vocabulary_list_json_with_count() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.vocabularies(&counted_view(), &count_cost_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains(&format!("\"note\":\"{COUNT_COST}\"")),
        "json _meta.note must carry the count_cost disclosure text; got:\n{out}"
    );
    assert!(
        out.contains("\"nodes\":") && out.contains("\"depth\":"),
        "json data items must carry nodes/depth keys when every item has a count; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

/// CSV render of the counted fixture. Locks the 10-column counted-default
/// header (`...,label,nodes,depth`) auto-showing because every item carries a
/// count.
#[test]
fn vocabulary_list_csv_with_count() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.vocabularies(&counted_view(), &count_cost_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    assert!(
        stdout.starts_with("name,iri,label_en,label_de,label_fr,label_it,label_rm,label,nodes,depth\n"),
        "CSV with-count header must add nodes,depth; got:\n{stdout}"
    );
    insta::assert_snapshot!("vocabulary_list_csv_with_count_stdout", stdout);
    insta::assert_snapshot!("vocabulary_list_csv_with_count_stderr", buf_to_string(&err_buf));
}

/// TSV render of the counted fixture. Same auto-expanding column shape as CSV
/// but tab-separated and unquoted.
#[test]
fn vocabulary_list_tsv_with_count() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = TsvRenderer::with_writers(out_w, err_w);
    r.vocabularies(&counted_view(), &count_cost_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    assert!(
        stdout.starts_with("name\tiri\tlabel_en\tlabel_de\tlabel_fr\tlabel_it\tlabel_rm\tlabel\tnodes\tdepth\n"),
        "TSV with-count header must add nodes,depth; got:\n{stdout}"
    );
    insta::assert_snapshot!("vocabulary_list_tsv_with_count_stdout", stdout);
    insta::assert_snapshot!("vocabulary_list_tsv_with_count_stderr", buf_to_string(&err_buf));
}

// ── all-failed-counted fixture ───────────────────────────────────────────────

/// CSV render of the all-failed-counted fixture. Locks that the plain 8-column
/// default header is kept (NOT the counted 10-column default) — the dynamic
/// column-set rule keys off whether any item actually carries a count, not
/// merely `view.counted`.
#[test]
fn vocabulary_list_csv_count_all_failed() {
    let (out_buf, out_w) = shared_buf();
    let (err_buf, err_w) = shared_buf();
    let mut r = CsvRenderer::with_writers(out_w, err_w);
    r.vocabularies(&all_failed_view(), &count_cost_all_failed_meta()).unwrap();
    let stdout = buf_to_string(&out_buf);
    let header_line = stdout.lines().next().unwrap();
    assert!(
        header_line == "name,iri,label_en,label_de,label_fr,label_it,label_rm,label",
        "CSV header must stay the plain 8-column default when every --count \
         fetch failed (no item carries a count); got: {header_line:?}"
    );
    insta::assert_snapshot!("vocabulary_list_csv_count_all_failed_stdout", stdout);
    insta::assert_snapshot!("vocabulary_list_csv_count_all_failed_stderr", buf_to_string(&err_buf));
}

// ── empty fixture × prose ─────────────────────────────────────────────────────

/// Prose render of the empty fixture. Locks the `(0)` header count and that
/// the ADR-0007 footer is still present.
#[test]
fn vocabulary_list_prose_empty() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.vocabularies(&empty_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(out.contains("(0)"), "prose empty must show '(0)' count; got:\n{out}");
    assert!(
        out.contains("[anonymous on"),
        "prose empty must still have disclosure footer; got:\n{out}"
    );
    insta::assert_snapshot!(out);
}

// ── filter fixture × prose ────────────────────────────────────────────────────

/// Prose render with an active filter. The main fixture has 4 items; filtering
/// by "epo" yields 1 match (`epoch`). Total stays at 4. Locks the "(1 of 4
/// matching \"epo\")" count line.
#[test]
fn vocabulary_list_prose_filter() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.vocabularies(&filter_view(), &anon_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("matching \"epo\""),
        "prose filter must contain 'matching \"epo\"'; got:\n{out}"
    );
    assert!(out.contains("of 4"), "prose filter must show 'of 4' total; got:\n{out}");
    insta::assert_snapshot!(out);
}
