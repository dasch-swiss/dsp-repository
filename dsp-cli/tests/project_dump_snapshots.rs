//! Snapshot tests for the `project_dump` and `project_dump_deleted` renderer cells.
//!
//! Drives the renderer methods directly by constructing fixed fixtures and a
//! fully synthetic `MetaContext` — no real tokens, no real email, no
//! `Utc::now()` calls. Deterministic.
//!
//! Coverage (one snapshot per (noun, format) cell):
//! - `project_dump` × 5 formats (prose, json, lines, csv, tsv) — `cleaned_up: false`, `reused:
//!   false`
//! - `project_dump` × prose — `cleaned_up: true`  (second-line cleanup disclosure)
//! - `project_dump` × json  — `cleaned_up: true`  (`cleaned_up: true` in JSON data)
//! - `project_dump` reused variant × prose + json (with fixed `created_at`)
//! - `project_dump_deleted` × 5 formats — `deleted: true`
//! - `project_dump_deleted` × prose + json — `deleted: false` with note (probe case)
//! - `project_dump_deleted` × prose + json — `deleted: false` with foreign-slot note
//! - `project_dump_conflict_default_refusal` × json — `Diagnostic::Conflict` from default-mode
//!   cross-project guard
//! - `project_dump_conflict_default_refusal` × prose — stdout empty (prose errors go to stderr)
//! - `project_dump_conflict_replace_no_flag` × json — `Diagnostic::Conflict` from
//!   replace-without-flag guard
//! - `project_dump_conflict_replace_no_flag` × prose — stdout empty

use dsp_cli::diagnostic::Diagnostic;
use dsp_cli::render::csv::CsvRenderer;
use dsp_cli::render::dump::{DumpDeleteOutcome, DumpOutcome};
use dsp_cli::render::json::JsonRenderer;
use dsp_cli::render::lines::LinesRenderer;
use dsp_cli::render::prose::ProseRenderer;
use dsp_cli::render::tsv::TsvRenderer;
use dsp_cli::render::{MetaContext, Renderer};

mod support;
use support::{buf_to_string, shared_buf};

// ── fixtures ──────────────────────────────────────────────────────────────────

/// Fixed `DumpOutcome` with `cleaned_up: false`, `reused: false`.
fn dump_outcome_not_cleaned() -> DumpOutcome {
    DumpOutcome {
        path: std::path::PathBuf::from("./0001-20260529T120000Z.zip"),
        bytes: 12345678,
        cleaned_up: false,
        reused: false,
        created_at: None,
    }
}

/// Fixed `DumpOutcome` with `cleaned_up: true` — for cleanup-disclosure coverage.
fn dump_outcome_cleaned() -> DumpOutcome {
    DumpOutcome {
        path: std::path::PathBuf::from("./0001-20260529T120000Z.zip"),
        bytes: 12345678,
        cleaned_up: true,
        reused: false,
        created_at: None,
    }
}

/// Fixed timestamp used in reused-variant fixtures.
/// `2026-05-20T14:03:00Z` — deterministic, not a real dump timestamp.
fn fixed_created_at() -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    chrono::Utc.with_ymd_and_hms(2026, 5, 20, 14, 3, 0).unwrap()
}

/// Fixed `DumpOutcome` with `reused: true` and a known `created_at`.
fn dump_outcome_reused() -> DumpOutcome {
    DumpOutcome {
        path: std::path::PathBuf::from("./0001-20260529T120000Z.zip"),
        bytes: 12345678,
        cleaned_up: false,
        reused: true,
        created_at: Some(fixed_created_at()),
    }
}

/// Fixed `DumpDeleteOutcome` with `deleted: true`.
fn dump_delete_outcome_deleted() -> DumpDeleteOutcome {
    DumpDeleteOutcome { deleted: true, note: None }
}

/// Fixed `DumpDeleteOutcome` with `deleted: false` and a probe note.
fn dump_delete_outcome_not_deleted() -> DumpDeleteOutcome {
    DumpDeleteOutcome {
        deleted: false,
        note: Some(
            "no dump existed to delete; a probe created an in-progress dump probe-id-999 that will complete server-side"
                .to_string(),
        ),
    }
}

/// Fixed `DumpDeleteOutcome` with `deleted: false` and a foreign-slot note.
///
/// This is case (b) from the `DumpDeleteOutcome` doc: the single dump slot is
/// held by a different project's dump, so the delete is a no-op.
fn dump_delete_outcome_other_project() -> DumpDeleteOutcome {
    DumpDeleteOutcome {
        deleted: false,
        note: Some(
            "no dump for the requested project to delete; the server's single dump slot is held by a different project (http://rdfh.ch/projects/0002)".to_string(),
        ),
    }
}

/// Fully synthetic `MetaContext` — never derived from a real token or email.
/// Uses dsp-cli/ADR-0007 vocabulary: "authenticated" (cache token, no user in this fixture).
fn dump_meta() -> MetaContext {
    MetaContext {
        server_label: "https://api.test.dasch.swiss".to_string(),
        auth_state: "authenticated".to_string(),
        filter_warning: None,
        count_caveat: None,
        count_cost: None,
    }
}

// ── cells: cleaned_up=false ───────────────────────────────────────────────────

#[test]
fn project_dump_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.project_dump(&dump_outcome_not_cleaned(), &dump_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn project_dump_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.project_dump(&dump_outcome_not_cleaned(), &dump_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn project_dump_lines() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.project_dump(&dump_outcome_not_cleaned(), &dump_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn project_dump_csv() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.project_dump(&dump_outcome_not_cleaned(), &dump_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn project_dump_tsv() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.project_dump(&dump_outcome_not_cleaned(), &dump_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── cells: cleaned_up=true (cleanup disclosure) ───────────────────────────────

/// Prose with cleanup: must emit the second line "Cleaned up the server-side dump."
#[test]
fn project_dump_prose_cleaned_up() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.project_dump(&dump_outcome_cleaned(), &dump_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("Cleaned up the server-side dump."),
        "prose cleaned_up=true must include cleanup disclosure; got: {out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON with cleanup: must include `"cleaned_up": true` in the data object.
#[test]
fn project_dump_json_cleaned_up() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.project_dump(&dump_outcome_cleaned(), &dump_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("\"cleaned_up\":true"),
        "json cleaned_up=true must include cleaned_up field; got: {out}"
    );
    insta::assert_snapshot!(out);
}

// ── cells: reused=true (adopt disclosure) ────────────────────────────────────

/// Prose with reused=true+created_at: must prepend the "Downloaded existing dump" line.
#[test]
fn project_dump_prose_reused() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.project_dump(&dump_outcome_reused(), &dump_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("Downloaded existing dump"),
        "prose reused=true must include 'Downloaded existing dump'; got: {out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON with reused=true: must include `"reused": true` and `"created_at"` in data.
#[test]
fn project_dump_json_reused() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.project_dump(&dump_outcome_reused(), &dump_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("\"reused\":true"),
        "json reused=true must include reused field; got: {out}"
    );
    insta::assert_snapshot!(out);
}

// ── cells: project_dump_deleted (deleted=true) ───────────────────────────────

#[test]
fn project_dump_deleted_prose() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.project_dump_deleted(&dump_delete_outcome_deleted(), &dump_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn project_dump_deleted_json() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.project_dump_deleted(&dump_delete_outcome_deleted(), &dump_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn project_dump_deleted_lines() {
    let (buf, w) = shared_buf();
    let mut r = LinesRenderer::with_writer(w);
    r.project_dump_deleted(&dump_delete_outcome_deleted(), &dump_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn project_dump_deleted_csv() {
    let (buf, w) = shared_buf();
    let mut r = CsvRenderer::with_writer(w);
    r.project_dump_deleted(&dump_delete_outcome_deleted(), &dump_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn project_dump_deleted_tsv() {
    let (buf, w) = shared_buf();
    let mut r = TsvRenderer::with_writer(w);
    r.project_dump_deleted(&dump_delete_outcome_deleted(), &dump_meta()).unwrap();
    insta::assert_snapshot!(buf_to_string(&buf));
}

// ── cells: project_dump_deleted (deleted=false, probe note) ──────────────────

/// Prose with deleted=false: must print the note instead of the delete confirmation.
#[test]
fn project_dump_deleted_prose_probe() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.project_dump_deleted(&dump_delete_outcome_not_deleted(), &dump_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("no dump existed"),
        "prose deleted=false must print the note; got: {out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON with deleted=false: must include `"deleted": false` and `"note"` in data.
#[test]
fn project_dump_deleted_json_probe() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.project_dump_deleted(&dump_delete_outcome_not_deleted(), &dump_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("\"deleted\":false"),
        "json deleted=false must include deleted field; got: {out}"
    );
    assert!(
        out.contains("\"note\""),
        "json deleted=false must include note field; got: {out}"
    );
    insta::assert_snapshot!(out);
}

// ── cells: project_dump_deleted (deleted=false, foreign-slot note) ────────────

/// Prose with deleted=false (foreign-slot case): must print the foreign-project note.
#[test]
fn project_dump_deleted_prose_other_project() {
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.project_dump_deleted(&dump_delete_outcome_other_project(), &dump_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("different project"),
        "prose deleted=false foreign-slot must mention 'different project'; got: {out}"
    );
    assert!(
        out.contains("http://rdfh.ch/projects/0002"),
        "prose deleted=false foreign-slot must name the foreign project IRI; got: {out}"
    );
    insta::assert_snapshot!(out);
}

/// JSON with deleted=false (foreign-slot case): must include `"deleted": false`,
/// `"note"` naming the foreign project.
#[test]
fn project_dump_deleted_json_other_project() {
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.project_dump_deleted(&dump_delete_outcome_other_project(), &dump_meta())
        .unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("\"deleted\":false"),
        "json deleted=false foreign-slot must include deleted field; got: {out}"
    );
    assert!(
        out.contains("\"note\""),
        "json deleted=false foreign-slot must include note field; got: {out}"
    );
    assert!(
        out.contains("0002"),
        "json deleted=false foreign-slot note must name the foreign project IRI; got: {out}"
    );
    insta::assert_snapshot!(out);
}

// ── cells: error envelopes for cross-project guard refusals ──────────────────

/// Default-mode cross-project refusal — JSON error envelope.
///
/// `Diagnostic::Conflict` → kind `conflict`, exit_code `1`. Must name the
/// foreign project IRI in the message. (Prose errors go to stderr in
/// production; the prose snapshot below confirms stdout is empty.)
#[test]
fn project_dump_conflict_default_refusal_json() {
    let foreign_iri = "http://rdfh.ch/projects/0002";
    let diag = Diagnostic::Conflict(format!(
        "no dump exists for the requested project; the server holds a single \
dump and it currently belongs to a different project ({foreign_iri}). Re-run \
with --replace --discard-other-project to discard that dump and create this \
project's, or wait for it to be removed."
    ));
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.diagnostic(&diag, &dump_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("\"kind\":\"conflict\""),
        "json conflict must have kind=conflict; got: {out}"
    );
    assert!(
        out.contains(foreign_iri),
        "json conflict must name the foreign project IRI; got: {out}"
    );
    insta::assert_snapshot!(out);
}

/// Default-mode cross-project refusal — prose stdout must be empty.
///
/// Prose errors are emitted to stderr by `main.rs`; the renderer's stdout
/// writer receives nothing. This snapshot locks down that contract.
#[test]
fn project_dump_conflict_default_refusal_prose() {
    let foreign_iri = "http://rdfh.ch/projects/0002";
    let diag = Diagnostic::Conflict(format!(
        "no dump exists for the requested project; the server holds a single \
dump and it currently belongs to a different project ({foreign_iri}). Re-run \
with --replace --discard-other-project to discard that dump and create this \
project's, or wait for it to be removed."
    ));
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.diagnostic(&diag, &dump_meta()).unwrap();
    let out = buf_to_string(&buf);
    insta::assert_snapshot!(out);
}

/// Replace-without-flag cross-project refusal — JSON error envelope.
#[test]
fn project_dump_conflict_replace_no_flag_json() {
    let foreign_iri = "http://rdfh.ch/projects/0002";
    let diag = Diagnostic::Conflict(format!(
        "the server's single dump slot is held by a different project \
({foreign_iri}); re-run with --replace --discard-other-project to discard that \
project's dump and create this one's"
    ));
    let (buf, w) = shared_buf();
    let mut r = JsonRenderer::with_writer(w);
    r.diagnostic(&diag, &dump_meta()).unwrap();
    let out = buf_to_string(&buf);
    assert!(
        out.contains("\"kind\":\"conflict\""),
        "json conflict must have kind=conflict; got: {out}"
    );
    assert!(
        out.contains(foreign_iri),
        "json conflict must name the foreign project IRI; got: {out}"
    );
    insta::assert_snapshot!(out);
}

/// Replace-without-flag cross-project refusal — prose stdout must be empty.
#[test]
fn project_dump_conflict_replace_no_flag_prose() {
    let foreign_iri = "http://rdfh.ch/projects/0002";
    let diag = Diagnostic::Conflict(format!(
        "the server's single dump slot is held by a different project \
({foreign_iri}); re-run with --replace --discard-other-project to discard that \
project's dump and create this one's"
    ));
    let (buf, w) = shared_buf();
    let mut r = ProseRenderer::with_writer(w);
    r.diagnostic(&diag, &dump_meta()).unwrap();
    let out = buf_to_string(&buf);
    insta::assert_snapshot!(out);
}
