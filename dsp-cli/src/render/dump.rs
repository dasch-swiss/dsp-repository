//! Dump renderer-input shapes — typed values passed to `Renderer::project_dump`
//! and `ProgressReporter::report`.
//!
//! These are render-layer types, not domain model types. They live here rather
//! than in `src/model/` because they flow from the action layer into the
//! renderer/reporter, not across the client→action boundary. See ADR-0008 for
//! the layer split.
//!
//! Note on cross-layer dependency: `DumpEvent::Polling` carries `DumpStatus`, a
//! `model/` type, so `render/` depends on `model/`. That direction is allowed by
//! ADR-0008 (render may reference models). `InProgress`/`Completed`/`Failed` are
//! generic task-state words — not DSP-API-divergent vocabulary (the ADR-0001
//! boundary is about `ontology`/`class`/`property` terms). Accepted as a shared
//! model type.

use chrono::{DateTime, Utc};

use crate::model::DumpStatus;

/// Final result of a completed dump, passed from the action layer to the
/// `Renderer`.
pub struct DumpOutcome {
    /// Filesystem path where the dump file was written.
    pub path: std::path::PathBuf,
    /// Number of bytes written.
    pub bytes: u64,
    /// `true` when the server-side dump was deleted after download
    /// (i.e. `--cleanup` succeeded).
    pub cleaned_up: bool,
    /// `true` when an existing server-side dump was adopted (idempotent
    /// re-download) rather than a fresh dump being triggered.
    pub reused: bool,
    /// When the server-side dump was originally created, if known.
    /// May be `None` if the server omitted `createdAt` or the timestamp
    /// failed to parse.
    pub created_at: Option<DateTime<Utc>>,
}

/// Final result of a delete operation, passed from the action layer to the
/// `Renderer` via `Renderer::project_dump_deleted`.
///
/// `deleted: false` has two distinct meanings, discriminated by `note`:
/// - **(a) Probe case**: no existing dump was found — `create_project_dump`
///   created a new in-progress dump as a probe side effect. `note` describes
///   the probe dump id.
/// - **(b) Foreign-slot case**: the server's single dump slot is occupied by a
///   different project's dump. The delete is a no-op (we never remove another
///   project's dump). `note` names the occupying project's IRI.
///
/// Delete *failures* are returned as `Err(Diagnostic)` from the action, never
/// as a `DumpDeleteOutcome`.
pub struct DumpDeleteOutcome {
    /// `true` when an existing dump was actually removed.
    /// `false` when no completed/failed dump existed (see doc-comment above for
    /// the two `false` cases).
    pub deleted: bool,
    /// Human-readable note for the `deleted: false` cases.
    /// `None` when `deleted: true`.
    pub note: Option<String>,
}

/// A progress event during a dump run, passed from the action layer to the
/// `ProgressReporter` (stderr only).
///
/// `DumpEvent::Done` is consumed **only** by the `ProgressReporter` (stderr);
/// it never reaches the `Renderer`. The final byte count reaches the renderer
/// via `DumpOutcome.bytes`.
pub enum DumpEvent {
    /// The dump was successfully triggered on the server.
    Triggered {
        /// Server-assigned dump ID.
        id: String,
    },
    /// A polling tick while waiting for the dump to complete.
    Polling {
        /// Logical elapsed time in seconds (explicit accumulator, not wall
        /// clock). The first `Polling` event has `elapsed_secs = 0`.
        elapsed_secs: u64,
        /// Current task status as reported by the server.
        status: DumpStatus,
    },
    /// The dump is complete on the server and download has started.
    Downloading,
    /// The download finished. Consumed only by the `ProgressReporter`;
    /// the renderer receives the byte count via `DumpOutcome.bytes` instead.
    Done {
        /// Number of bytes received from the server.
        bytes: u64,
    },
    /// An existing completed or in-progress dump was found and is being
    /// adopted (idempotent re-download, default mode).
    Adopting {
        /// The existing dump's server-assigned ID.
        id: String,
    },
    /// An existing dump is being deleted before a fresh one is triggered
    /// (`--replace` mode).
    Deleting {
        /// The dump ID being deleted.
        id: String,
    },
    /// `--delete` was requested but no completed/failed dump existed; the
    /// `create_project_dump` probe created a new in-progress dump. The dump
    /// will complete server-side without being downloaded.
    ProbeCreated {
        /// The ID of the newly-created in-progress dump.
        id: String,
    },
    /// `--replace --discard-other-project` is discarding an existing dump that
    /// belongs to a **different** project to free the single server-wide dump slot.
    DiscardingOtherProjectDump {
        /// The foreign dump's id being discarded.
        id: String,
        /// The IRI of the project that dump belongs to (server-reported value).
        project_iri: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_outcome_construction() {
        let outcome = DumpOutcome {
            path: std::path::PathBuf::from("./0001-20260529T120000Z.zip"),
            bytes: 1024,
            cleaned_up: true,
            reused: false,
            created_at: None,
        };
        assert_eq!(outcome.bytes, 1024);
        assert!(outcome.cleaned_up);
        assert!(!outcome.reused);
        assert!(outcome.created_at.is_none());
        assert_eq!(
            outcome.path,
            std::path::PathBuf::from("./0001-20260529T120000Z.zip")
        );
    }

    #[test]
    fn dump_outcome_reused_with_created_at() {
        use chrono::TimeZone;
        let ts = Utc.with_ymd_and_hms(2026, 5, 20, 14, 3, 0).unwrap();
        let outcome = DumpOutcome {
            path: std::path::PathBuf::from("./0001-20260529T120000Z.zip"),
            bytes: 999,
            cleaned_up: false,
            reused: true,
            created_at: Some(ts),
        };
        assert!(outcome.reused);
        assert_eq!(outcome.created_at, Some(ts));
    }

    #[test]
    fn dump_delete_outcome_construction() {
        let deleted = DumpDeleteOutcome {
            deleted: true,
            note: None,
        };
        assert!(deleted.deleted);
        assert!(deleted.note.is_none());

        // Case (a): probe — create_project_dump created a new in-progress dump.
        let probe = DumpDeleteOutcome {
            deleted: false,
            note: Some("no dump existed; a probe created an in-progress dump probe-id-42 that will complete server-side".to_string()),
        };
        assert!(!probe.deleted);
        assert!(probe.note.is_some());
        assert!(probe.note.unwrap().contains("probe-id-42"));

        // Case (b): foreign-slot — the single dump slot is held by a different project.
        let foreign_slot = DumpDeleteOutcome {
            deleted: false,
            note: Some(
                "no dump for the requested project to delete; the server's single dump slot is held by a different project (http://rdfh.ch/projects/0002)".to_string(),
            ),
        };
        assert!(!foreign_slot.deleted);
        let note = foreign_slot.note.unwrap();
        assert!(note.contains("0002"), "note must name the foreign project");
    }

    #[test]
    fn dump_event_variants_construct() {
        let triggered = DumpEvent::Triggered {
            id: "dump-id-42".into(),
        };
        let polling = DumpEvent::Polling {
            elapsed_secs: 0,
            status: DumpStatus::InProgress,
        };
        let downloading = DumpEvent::Downloading;
        let done = DumpEvent::Done { bytes: 2048 };
        let adopting = DumpEvent::Adopting {
            id: "existing-id".into(),
        };
        let deleting = DumpEvent::Deleting {
            id: "del-id".into(),
        };
        let probe_created = DumpEvent::ProbeCreated {
            id: "probe-id".into(),
        };
        let discarding_other = DumpEvent::DiscardingOtherProjectDump {
            id: "foreign-id".into(),
            project_iri: "http://rdfh.ch/projects/0002".into(),
        };

        // Pattern-match to verify enum arms are reachable.
        match triggered {
            DumpEvent::Triggered { id } => assert_eq!(id, "dump-id-42"),
            _ => panic!("unexpected variant"),
        }
        match polling {
            DumpEvent::Polling {
                elapsed_secs,
                status,
            } => {
                assert_eq!(elapsed_secs, 0);
                assert_eq!(status, DumpStatus::InProgress);
            }
            _ => panic!("unexpected variant"),
        }
        assert!(matches!(downloading, DumpEvent::Downloading));
        match done {
            DumpEvent::Done { bytes } => assert_eq!(bytes, 2048),
            _ => panic!("unexpected variant"),
        }
        match adopting {
            DumpEvent::Adopting { id } => assert_eq!(id, "existing-id"),
            _ => panic!("unexpected variant"),
        }
        match deleting {
            DumpEvent::Deleting { id } => assert_eq!(id, "del-id"),
            _ => panic!("unexpected variant"),
        }
        match probe_created {
            DumpEvent::ProbeCreated { id } => assert_eq!(id, "probe-id"),
            _ => panic!("unexpected variant"),
        }
        match discarding_other {
            DumpEvent::DiscardingOtherProjectDump { id, project_iri } => {
                assert_eq!(id, "foreign-id");
                assert_eq!(project_iri, "http://rdfh.ch/projects/0002");
            }
            _ => panic!("unexpected variant"),
        }
    }
}
