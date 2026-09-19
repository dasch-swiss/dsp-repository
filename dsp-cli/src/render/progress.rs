//! Progress reporter layer — stderr progress lines during a dump run.
//!
//! `ProgressReporter` is a separate trait from `Renderer` because:
//! - The renderer owns a single stdout sink; progress events belong on stderr.
//! - Two impls (Human, JSON) vs five renderers keeps it right-sized — stderr is non-data output so
//!   prose/lines/csv/tsv all share the human reporter.
//! - Isolates the novel "multi-line polling progress to stderr" concern from the renderer's stdout
//!   concerns (endorsed in the Step 6 rationale, ADR-0008).
//!
//! Returning `Result` is intentional: stderr writes can fail (e.g. the other
//! end of a pipe closed unexpectedly). The action treats a stderr-write failure
//! as fatal, consistent with the renderer's own `Write` calls.

use std::io::{self, Write};

use crate::diagnostic::Diagnostic;
use crate::render::DumpEvent;

/// Reports dump progress events to stderr (or any injected `Write` sink).
///
/// `report` is called once per event during a dump run. The action treats any
/// returned `Err` as fatal (stderr-write failure is not silently ignored, for
/// the same reason the renderer's `writeln!` failures are not ignored).
pub trait ProgressReporter {
    /// Emit a progress event.
    ///
    /// Returning `Result` is intentional — stderr writes can fail and the
    /// action treats a stderr-write failure as fatal, consistent with the
    /// renderer's own `Write` calls.
    fn report(&mut self, event: &DumpEvent) -> Result<(), Diagnostic>;
}

// ── HumanProgress ─────────────────────────────────────────────────────────────

/// Human-readable progress reporter — one line per event to stderr.
///
/// `DumpEvent::Done` emits **nothing** because the renderer owns the final
/// stdout line (the path + byte count). This mirrors the design decision in
/// `render/dump.rs`: `Done` is consumed by the reporter only to let it flush;
/// the visible confirmation is `Renderer::project_dump`.
pub struct HumanProgress {
    err: Box<dyn Write>,
}

impl HumanProgress {
    /// Creates a reporter writing to `io::stderr()`.
    pub fn new() -> Self {
        Self { err: Box::new(io::stderr()) }
    }

    /// Creates a reporter writing to an arbitrary `Write` sink (used in tests).
    pub fn with_writer(w: impl Write + 'static) -> Self {
        Self { err: Box::new(w) }
    }
}

impl Default for HumanProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for HumanProgress {
    fn report(&mut self, event: &DumpEvent) -> Result<(), Diagnostic> {
        match event {
            DumpEvent::Triggered { id } => {
                writeln!(self.err, "Triggered dump {id}.")?;
            }
            DumpEvent::Polling { elapsed_secs, status } => {
                writeln!(
                    self.err,
                    "polling… {elapsed_secs}s elapsed ({status})",
                    status = status.as_str(),
                )?;
            }
            DumpEvent::Downloading => {
                writeln!(self.err, "Downloading…")?;
            }
            // Done emits nothing — the renderer owns the final stdout line.
            DumpEvent::Done { .. } => {}
            DumpEvent::Adopting { id } => {
                writeln!(self.err, "Found existing dump {id}; adopting it.")?;
            }
            DumpEvent::Deleting { id } => {
                writeln!(self.err, "Deleting dump {id}…")?;
            }
            DumpEvent::ProbeCreated { id } => {
                writeln!(
                    self.err,
                    "No dump existed; a probe created a new in-progress dump {id} (it will complete server-side)."
                )?;
            }
            DumpEvent::DiscardingOtherProjectDump { project_iri, .. } => {
                writeln!(
                    self.err,
                    "\u{26a0} Discarding the existing dump for a different project ({project_iri}) \
\u{2014} the DSP-API holds one dump server-wide."
                )?;
            }
        }
        Ok(())
    }
}

// ── JsonProgress ──────────────────────────────────────────────────────────────

/// JSON progress reporter — one compact JSON object per event to stderr.
///
/// Each event is serialised with `serde_json` and written as a single
/// newline-terminated line:
///
/// ```text
/// {"event":"triggered","id":"abc123"}
/// {"event":"polling","elapsed_s":12,"status":"in_progress"}
/// {"event":"downloading"}
/// {"event":"done","bytes":12345}
/// ```
///
/// The `Done` event is emitted (unlike `HumanProgress`) so JSON consumers
/// can parse a complete event stream without relying solely on the final
/// stdout object from the renderer.
pub struct JsonProgress {
    err: Box<dyn Write>,
}

impl JsonProgress {
    /// Creates a reporter writing to `io::stderr()`.
    pub fn new() -> Self {
        Self { err: Box::new(io::stderr()) }
    }

    /// Creates a reporter writing to an arbitrary `Write` sink (used in tests).
    pub fn with_writer(w: impl Write + 'static) -> Self {
        Self { err: Box::new(w) }
    }
}

impl Default for JsonProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for JsonProgress {
    fn report(&mut self, event: &DumpEvent) -> Result<(), Diagnostic> {
        let obj = match event {
            DumpEvent::Triggered { id } => {
                serde_json::json!({"event": "triggered", "id": id})
            }
            DumpEvent::Polling { elapsed_secs, status } => {
                serde_json::json!({
                    "event": "polling",
                    "elapsed_s": elapsed_secs,
                    "status": status.as_str(),
                })
            }
            DumpEvent::Downloading => {
                serde_json::json!({"event": "downloading"})
            }
            DumpEvent::Done { bytes } => {
                serde_json::json!({"event": "done", "bytes": bytes})
            }
            DumpEvent::Adopting { id } => {
                serde_json::json!({"event": "adopting", "id": id})
            }
            DumpEvent::Deleting { id } => {
                serde_json::json!({"event": "deleting", "id": id})
            }
            DumpEvent::ProbeCreated { id } => {
                serde_json::json!({"event": "probe_created", "id": id})
            }
            DumpEvent::DiscardingOtherProjectDump { id, project_iri } => {
                serde_json::json!({
                    "event": "discarding_other_project_dump",
                    "id": id,
                    "project_iri": project_iri,
                })
            }
        };
        let line =
            serde_json::to_string(&obj).map_err(|e| Diagnostic::Internal(format!("json serialisation error: {e}")))?;
        writeln!(self.err, "{line}")?;
        Ok(())
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::model::DumpStatus;

    // ── shared buffer ─────────────────────────────────────────────────────────

    /// `Write` wrapper around a shared `Rc<RefCell<Vec<u8>>>` so we can read
    /// the captured bytes after the reporter is dropped. Mirrors the pattern
    /// used in `tests/auth_snapshots.rs`.
    struct SharedBuf(Rc<RefCell<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0.borrow_mut().flush()
        }
    }

    fn shared_buf() -> (Rc<RefCell<Vec<u8>>>, SharedBuf) {
        let buf = Rc::new(RefCell::new(Vec::<u8>::new()));
        let writer = SharedBuf(Rc::clone(&buf));
        (buf, writer)
    }

    fn buf_to_string(buf: &Rc<RefCell<Vec<u8>>>) -> String {
        String::from_utf8(buf.borrow().clone()).expect("output must be valid UTF-8")
    }

    fn collect_human(events: &[DumpEvent]) -> String {
        let (buf, w) = shared_buf();
        let mut r = HumanProgress::with_writer(w);
        for e in events {
            r.report(e).expect("report must not fail in tests");
        }
        buf_to_string(&buf)
    }

    fn collect_json(events: &[DumpEvent]) -> String {
        let (buf, w) = shared_buf();
        let mut r = JsonProgress::with_writer(w);
        for e in events {
            r.report(e).expect("report must not fail in tests");
        }
        buf_to_string(&buf)
    }

    // ── HumanProgress ─────────────────────────────────────────────────────────

    #[test]
    fn human_triggered_formats_correctly() {
        let out = collect_human(&[DumpEvent::Triggered { id: "abc123".into() }]);
        assert_eq!(out.trim(), "Triggered dump abc123.");
    }

    #[test]
    fn human_polling_in_progress_formats_correctly() {
        let out = collect_human(&[DumpEvent::Polling { elapsed_secs: 12, status: DumpStatus::InProgress }]);
        assert_eq!(out.trim(), "polling… 12s elapsed (in_progress)");
    }

    #[test]
    fn human_polling_completed_formats_correctly() {
        let out = collect_human(&[DumpEvent::Polling { elapsed_secs: 45, status: DumpStatus::Completed }]);
        assert_eq!(out.trim(), "polling… 45s elapsed (completed)");
    }

    #[test]
    fn human_polling_failed_formats_correctly() {
        let out = collect_human(&[DumpEvent::Polling { elapsed_secs: 99, status: DumpStatus::Failed }]);
        assert_eq!(out.trim(), "polling… 99s elapsed (failed)");
    }

    #[test]
    fn human_downloading_formats_correctly() {
        let out = collect_human(&[DumpEvent::Downloading]);
        assert_eq!(out.trim(), "Downloading…");
    }

    #[test]
    fn human_done_emits_nothing() {
        let out = collect_human(&[DumpEvent::Done { bytes: 12345 }]);
        assert_eq!(out, "", "HumanProgress must emit nothing for Done");
    }

    // ── JsonProgress ──────────────────────────────────────────────────────────

    fn parse_json_line(line: &str) -> serde_json::Value {
        serde_json::from_str(line).unwrap_or_else(|e| panic!("invalid JSON line {line:?}: {e}"))
    }

    #[test]
    fn json_triggered_fields_correct() {
        let out = collect_json(&[DumpEvent::Triggered { id: "abc123".into() }]);
        let v = parse_json_line(out.trim());
        assert_eq!(v["event"], "triggered");
        assert_eq!(v["id"], "abc123");
    }

    #[test]
    fn json_polling_in_progress_fields_correct() {
        let out = collect_json(&[DumpEvent::Polling { elapsed_secs: 12, status: DumpStatus::InProgress }]);
        let v = parse_json_line(out.trim());
        assert_eq!(v["event"], "polling");
        assert_eq!(v["elapsed_s"], 12);
        assert_eq!(v["status"], "in_progress");
    }

    #[test]
    fn json_polling_completed_fields_correct() {
        let out = collect_json(&[DumpEvent::Polling { elapsed_secs: 28, status: DumpStatus::Completed }]);
        let v = parse_json_line(out.trim());
        assert_eq!(v["event"], "polling");
        assert_eq!(v["elapsed_s"], 28);
        assert_eq!(v["status"], "completed");
    }

    #[test]
    fn json_downloading_fields_correct() {
        let out = collect_json(&[DumpEvent::Downloading]);
        let v = parse_json_line(out.trim());
        assert_eq!(v["event"], "downloading");
    }

    #[test]
    fn json_done_fields_correct() {
        let out = collect_json(&[DumpEvent::Done { bytes: 12345 }]);
        let v = parse_json_line(out.trim());
        assert_eq!(v["event"], "done");
        assert_eq!(v["bytes"], 12345);
    }

    // ── new events: Adopting, Deleting, ProbeCreated ──────────────────────────

    #[test]
    fn human_adopting_formats_correctly() {
        let out = collect_human(&[DumpEvent::Adopting { id: "existing-dump-id".into() }]);
        assert_eq!(out.trim(), "Found existing dump existing-dump-id; adopting it.");
    }

    #[test]
    fn human_deleting_formats_correctly() {
        let out = collect_human(&[DumpEvent::Deleting { id: "del-dump-id".into() }]);
        assert_eq!(out.trim(), "Deleting dump del-dump-id\u{2026}");
    }

    #[test]
    fn human_probe_created_formats_correctly() {
        let out = collect_human(&[DumpEvent::ProbeCreated { id: "probe-id-99".into() }]);
        assert_eq!(
            out.trim(),
            "No dump existed; a probe created a new in-progress dump probe-id-99 (it will complete server-side)."
        );
    }

    #[test]
    fn json_adopting_fields_correct() {
        let out = collect_json(&[DumpEvent::Adopting { id: "existing-dump-id".into() }]);
        let v = parse_json_line(out.trim());
        assert_eq!(v["event"], "adopting");
        assert_eq!(v["id"], "existing-dump-id");
    }

    #[test]
    fn json_deleting_fields_correct() {
        let out = collect_json(&[DumpEvent::Deleting { id: "del-dump-id".into() }]);
        let v = parse_json_line(out.trim());
        assert_eq!(v["event"], "deleting");
        assert_eq!(v["id"], "del-dump-id");
    }

    #[test]
    fn json_probe_created_fields_correct() {
        let out = collect_json(&[DumpEvent::ProbeCreated { id: "probe-id-99".into() }]);
        let v = parse_json_line(out.trim());
        assert_eq!(v["event"], "probe_created");
        assert_eq!(v["id"], "probe-id-99");
    }

    // ── new event: DiscardingOtherProjectDump ─────────────────────────────────

    #[test]
    fn human_discarding_other_project_dump_formats_correctly() {
        let out = collect_human(&[DumpEvent::DiscardingOtherProjectDump {
            id: "foreign-dump-id".into(),
            project_iri: "http://rdfh.ch/projects/0002".into(),
        }]);
        // Must mention the foreign project IRI and that it's the server-wide slot.
        assert!(
            out.contains("http://rdfh.ch/projects/0002"),
            "output must contain the foreign project IRI: {out:?}"
        );
        assert!(
            out.contains("different project"),
            "output must mention 'different project': {out:?}"
        );
    }

    #[test]
    fn json_discarding_other_project_dump_fields_correct() {
        let out = collect_json(&[DumpEvent::DiscardingOtherProjectDump {
            id: "foreign-dump-id".into(),
            project_iri: "http://rdfh.ch/projects/0002".into(),
        }]);
        let v = parse_json_line(out.trim());
        assert_eq!(v["event"], "discarding_other_project_dump");
        assert_eq!(v["id"], "foreign-dump-id");
        assert_eq!(v["project_iri"], "http://rdfh.ch/projects/0002");
        // Must NOT use camelCase "projectIri" key (ADR-0001).
        assert!(v.get("projectIri").is_none(), "JSON must not use camelCase 'projectIri'");
    }
}
