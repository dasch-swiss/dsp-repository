use crate::snapshot::ViolationReport;

/// Why a dump gave no snapshot.
#[derive(Debug, thiserror::Error)]
pub enum LiftError {
    /// The dump's files could not be read or parsed, or it holds no single
    /// project data graph.
    #[error("reading the dump: {0}")]
    Dump(String),
    /// The dump breaks `FORMAT.md`'s closed world (§11): every violation, listed.
    #[error(transparent)]
    Violations(#[from] ViolationReport),
    /// A rule the classifier guarantees did not hold for `subject`: a defect in
    /// this crate, not in the dump.
    #[error("internal error: {what} ({subject})")]
    Internal { what: &'static str, subject: String },
}
