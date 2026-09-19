//! Dump domain shapes — shared across the client → action boundary.
//!
//! Types here are the dsp-cli vocabulary for dump tasks. DSP-API wire types
//! (`DataTaskStatusApiResponse` etc.) live inside `src/client/http.rs` and are
//! never exposed above the client layer. See ADR-0001 and ADR-0008.

/// The status of a server-side dump task.
///
/// Does **not** derive `serde::Deserialize` — wire deserialization
/// (`"in_progress"` / `"completed"` / `"failed"` strings) happens via the
/// private wire DTO in `src/client/http.rs` (Step 5), keeping serde strictly at
/// the client boundary. See ADR-0001.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DumpStatus {
    InProgress,
    Completed,
    Failed,
}

impl DumpStatus {
    /// Return the canonical lower-case wire string for this status.
    ///
    /// This is the single source of truth for the status string form used in
    /// progress reporting (human and JSON). Parsing wire→enum happens in the
    /// HTTP client layer (`http.rs`) and is intentionally NOT delegated here.
    pub fn as_str(self) -> &'static str {
        match self {
            DumpStatus::InProgress => "in_progress",
            DumpStatus::Completed => "completed",
            DumpStatus::Failed => "failed",
        }
    }
}

/// A server-side dump task (boundary-translated from DSP-API's DataTask).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpTask {
    /// Unique identifier for the dump job (URL-safe, used verbatim in
    /// subsequent status/download/delete calls).
    pub id: String,
    pub status: DumpStatus,
    /// Only meaningful when `status == Failed`. The value is pre-truncated to
    /// ≤500 chars at the client boundary (the single truncation point in
    /// `HttpDspClient`); the Step 8 poll-loop `Failed` branch relies on this
    /// invariant — do not store un-truncated messages here.
    pub error_message: Option<String>,
    /// When the dump was created on the server, if known.
    ///
    /// Parsed best-effort from the `createdAt` field in the server response
    /// (RFC 3339). `None` if the field is absent or cannot be parsed —
    /// `created_at` is display-only, never load-bearing.
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// The outcome of `create_project_dump`, crossing the client→action boundary.
///
/// Decision matrix (per ADR-0001, ADR-0008, ADR-0012):
///
/// | Variant | Meaning | Typical action response |
/// |---|---|---|
/// | `Created(task)` | Fresh dump started; poll until done. | Poll → download. |
/// | `Exists { id }` | A dump for **this** project already exists. | Adopt / replace / error, depending on mode. |
/// | `ExistsForOtherProject { id, project_iri }` | The server's single dump slot is occupied by a **different** project's dump. | Refuse (default/delete) or discard + recreate (replace + `--discard-other-project`). |
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateDumpOutcome {
    /// A fresh dump was successfully triggered; `task.status` is `InProgress`.
    Created(DumpTask),
    /// A dump already exists; `id` is its server-assigned identifier.
    Exists {
        /// The ID of the pre-existing dump (URL-safe base64, verified).
        id: String,
    },
    /// A dump already exists, but it belongs to a **different** project than the
    /// one requested. The DSP-API holds a single dump server-wide; this signals
    /// that the slot is occupied by another project's dump.
    ///
    /// `project_iri` is the occupying project's IRI **as reported by the server**
    /// in the 409 conflict body — not user-supplied or domain-derived. Carrying an
    /// IRI *value* across layers is allowed under ADR-0001 (only the wire term
    /// "projectIri" and the word "export" are banned above `src/client/`).
    ExistsForOtherProject {
        /// The existing (foreign) dump's server-assigned id.
        id: String,
        /// The IRI of the project the existing dump belongs to.
        project_iri: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_status_equality() {
        assert_eq!(DumpStatus::InProgress, DumpStatus::InProgress);
        assert_ne!(DumpStatus::InProgress, DumpStatus::Completed);
        assert_ne!(DumpStatus::InProgress, DumpStatus::Failed);
        assert_ne!(DumpStatus::Completed, DumpStatus::Failed);
    }

    #[test]
    fn dump_task_construction_and_equality() {
        let task = DumpTask {
            id: "abc-123".into(),
            status: DumpStatus::InProgress,
            error_message: None,
            created_at: None,
        };
        let cloned = task.clone();
        assert_eq!(task, cloned);

        let failed = DumpTask {
            id: "def-456".into(),
            status: DumpStatus::Failed,
            error_message: Some("disk full".into()),
            created_at: None,
        };
        assert_ne!(task, failed);
        assert_eq!(failed.error_message.as_deref(), Some("disk full"));
    }

    #[test]
    fn create_dump_outcome_variants() {
        use chrono::TimeZone;
        let ts = chrono::Utc.with_ymd_and_hms(2026, 5, 20, 14, 3, 0).unwrap();
        let task = DumpTask {
            id: "task-1".into(),
            status: DumpStatus::InProgress,
            error_message: None,
            created_at: Some(ts),
        };
        let created = CreateDumpOutcome::Created(task.clone());
        assert!(matches!(created, CreateDumpOutcome::Created(_)));
        if let CreateDumpOutcome::Created(t) = &created {
            assert_eq!(t.id, "task-1");
            assert_eq!(t.created_at, Some(ts));
        }

        let exists = CreateDumpOutcome::Exists {
            id: "existing-id".into(),
        };
        assert!(matches!(exists, CreateDumpOutcome::Exists { .. }));
        if let CreateDumpOutcome::Exists { id } = &exists {
            assert_eq!(id, "existing-id");
        }

        // Equality
        assert_ne!(created, exists);
    }

    #[test]
    fn create_dump_outcome_exists_for_other_project_construction() {
        let other = CreateDumpOutcome::ExistsForOtherProject {
            id: "foreign-dump-id".into(),
            project_iri: "http://rdfh.ch/projects/0002".into(),
        };
        assert!(matches!(
            other,
            CreateDumpOutcome::ExistsForOtherProject { .. }
        ));
        if let CreateDumpOutcome::ExistsForOtherProject { id, project_iri } = &other {
            assert_eq!(id, "foreign-dump-id");
            assert_eq!(project_iri, "http://rdfh.ch/projects/0002");
        }

        // Verify derives: Clone, PartialEq, Eq, Debug
        let cloned = other.clone();
        assert_eq!(other, cloned);
        let different = CreateDumpOutcome::ExistsForOtherProject {
            id: "other-id".into(),
            project_iri: "http://rdfh.ch/projects/0003".into(),
        };
        assert_ne!(other, different);
        // Debug
        let s = format!("{other:?}");
        assert!(s.contains("ExistsForOtherProject"));
    }
}
