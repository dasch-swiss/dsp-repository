//! Snapshot tests for dump progress reporter output.
//!
//! Drives `HumanProgress` and `JsonProgress` through a fixed event sequence
//! and captures the full stderr output. All inputs (elapsed, IDs, bytes) are
//! explicit constants — no wall-clock reads, no env, fully deterministic.
//!
//! Sequence 1 (base): Triggered → Polling(12s,InProgress) → Polling(28s,InProgress)
//!                    → Downloading → Done(12345)
//!
//! Sequence 2 (adopt/delete/probe): Adopting → Deleting → ProbeCreated
//!
//! Sequence 3 (discard-other-project): DiscardingOtherProjectDump → Triggered → Downloading → Done
//!
//! Coverage:
//! - `human_progress_full_sequence` — full stderr text, prose format
//! - `json_progress_full_sequence` — full stderr NDJSON text, JSON format
//! - `human_progress_adopt_delete_probe_sequence` — adopt + delete + probe events, prose
//! - `json_progress_adopt_delete_probe_sequence` — adopt + delete + probe events, JSON
//! - `human_progress_discard_other_project_sequence` — discard-other-project flow, prose
//! - `json_progress_discard_other_project_sequence` — discard-other-project flow, JSON

use dsp_cli::model::DumpStatus;
use dsp_cli::render::DumpEvent;
use dsp_cli::render::progress::{HumanProgress, JsonProgress, ProgressReporter};

mod support;
use support::{buf_to_string, shared_buf};

// ── fixed event sequence ──────────────────────────────────────────────────────

fn fixed_events() -> Vec<DumpEvent> {
    vec![
        DumpEvent::Triggered { id: "abc123".into() },
        DumpEvent::Polling { elapsed_secs: 12, status: DumpStatus::InProgress },
        DumpEvent::Polling { elapsed_secs: 28, status: DumpStatus::InProgress },
        DumpEvent::Downloading,
        DumpEvent::Done { bytes: 12345 },
    ]
}

// ── fixed adopt/delete/probe sequence ────────────────────────────────────────

fn adopt_delete_probe_events() -> Vec<DumpEvent> {
    vec![
        DumpEvent::Adopting { id: "existing-dump-abc".into() },
        DumpEvent::Deleting { id: "dump-to-delete-xyz".into() },
        DumpEvent::ProbeCreated { id: "probe-created-999".into() },
    ]
}

// ── fixed discard-other-project sequence ─────────────────────────────────────

/// Sequence representing --replace --discard-other-project: the warning event,
/// then the new dump being triggered and downloaded.
fn discard_other_project_events() -> Vec<DumpEvent> {
    vec![
        DumpEvent::DiscardingOtherProjectDump {
            id: "foreign-dump-abc".into(),
            project_iri: "http://rdfh.ch/projects/0002".into(),
        },
        DumpEvent::Triggered { id: "new-dump-xyz".into() },
        DumpEvent::Downloading,
        DumpEvent::Done { bytes: 98765 },
    ]
}

// ── snapshot tests ────────────────────────────────────────────────────────────

#[test]
fn human_progress_full_sequence() {
    let (buf, w) = shared_buf();
    let mut r = HumanProgress::with_writer(w);
    for event in fixed_events() {
        r.report(&event).expect("report must not fail in tests");
    }
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn json_progress_full_sequence() {
    let (buf, w) = shared_buf();
    let mut r = JsonProgress::with_writer(w);
    for event in fixed_events() {
        r.report(&event).expect("report must not fail in tests");
    }
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn human_progress_adopt_delete_probe_sequence() {
    let (buf, w) = shared_buf();
    let mut r = HumanProgress::with_writer(w);
    for event in adopt_delete_probe_events() {
        r.report(&event).expect("report must not fail in tests");
    }
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn json_progress_adopt_delete_probe_sequence() {
    let (buf, w) = shared_buf();
    let mut r = JsonProgress::with_writer(w);
    for event in adopt_delete_probe_events() {
        r.report(&event).expect("report must not fail in tests");
    }
    insta::assert_snapshot!(buf_to_string(&buf));
}

#[test]
fn human_progress_discard_other_project_sequence() {
    let (buf, w) = shared_buf();
    let mut r = HumanProgress::with_writer(w);
    for event in discard_other_project_events() {
        r.report(&event).expect("report must not fail in tests");
    }
    let out = buf_to_string(&buf);
    // Must mention the foreign project IRI and the server-wide slot.
    assert!(
        out.contains("http://rdfh.ch/projects/0002"),
        "prose must contain the foreign project IRI; got: {out:?}"
    );
    assert!(
        out.contains("different project"),
        "prose must mention 'different project'; got: {out:?}"
    );
    insta::assert_snapshot!(out);
}

#[test]
fn json_progress_discard_other_project_sequence() {
    let (buf, w) = shared_buf();
    let mut r = JsonProgress::with_writer(w);
    for event in discard_other_project_events() {
        r.report(&event).expect("report must not fail in tests");
    }
    let out = buf_to_string(&buf);
    // Must use snake_case event key and project_iri (not camelCase projectIri).
    assert!(
        out.contains("discarding_other_project_dump"),
        "json must use 'discarding_other_project_dump' event key; got: {out:?}"
    );
    assert!(
        out.contains("project_iri"),
        "json must use snake_case 'project_iri' key (not 'projectIri'); got: {out:?}"
    );
    assert!(
        !out.contains("projectIri"),
        "json must NOT use camelCase 'projectIri' key; got: {out:?}"
    );
    insta::assert_snapshot!(out);
}
