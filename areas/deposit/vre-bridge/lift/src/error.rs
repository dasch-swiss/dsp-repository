use std::fmt;

use crate::guard::Violation;
use crate::snapshot::ViolationReport;

/// Why a dump gave no snapshot.
#[derive(Debug, thiserror::Error)]
pub enum LiftError {
    /// The dump's files could not be read or parsed, or it holds no single
    /// project data graph.
    #[error("reading the dump: {0}")]
    Dump(String),
    /// The public guard refused the project: every selected resource or value the
    /// anonymous user may not view, sorted by subject.
    #[error("{}", Refusal(.violations))]
    Refused { violations: Vec<Violation> },
    /// The dump breaks `FORMAT.md`'s closed world (§11): every violation, listed.
    #[error(transparent)]
    ClosedWorld(#[from] ViolationReport),
    /// A rule the classifier guarantees did not hold for `subject`: a defect in
    /// this crate, not in the dump.
    #[error("internal error: {what} ({subject})")]
    Internal { what: &'static str, subject: String },
}

/// A refusal's text: a count, then every violation on its own line.
struct Refusal<'v>(&'v [Violation]);

impl fmt::Display for Refusal<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "refused by the public guard: {} violation(s)", self.0.len())?;
        self.0.iter().try_for_each(|v| write!(f, "\n  {v}"))
    }
}
