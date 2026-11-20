//! What a refresh reports once it has ended: the outcome line and the metrics' values.

use std::time::Duration;

use vre_bridge_core::{Outcome, ServerId};

/// One ended refresh, or one removal. A busy dump slot is no outcome; only giving up on it is
/// reported, as a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshReport {
    pub project: String,
    /// The allowlist entry's server; `None` for [`Outcome::Removed`], whose entry is gone.
    pub server: Option<ServerId>,
    pub outcome: Outcome,
    pub duration: Duration,
    /// Set for [`Outcome::Refreshed`] only.
    pub sizes: Option<RefreshSizes>,
}

/// The bytes a successful refresh handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshSizes {
    /// The downloaded dump zip.
    pub dump: u64,
    /// The interim snapshot written to `sync`.
    pub snapshot: u64,
    /// Everything written to the `media` generation.
    pub media: u64,
}

/// Where a refresh's outcome goes.
pub trait Report: Send + Sync {
    fn refresh(&self, report: &RefreshReport);
}

/// The outcome line alone: info for a success, warning for a refusal, error for a failure.
#[derive(Debug, Clone, Copy, Default)]
pub struct LogReport;

impl Report for LogReport {
    fn refresh(&self, r: &RefreshReport) {
        // `tracing`'s level must be a constant, hence one call per level.
        macro_rules! outcome_line {
            ($level:ident, $message:literal) => {
                tracing::$level!(
                    project = %r.project,
                    server = r.server.as_ref().map(ServerId::as_str),
                    outcome = r.outcome.as_str(),
                    reason = r.outcome.reason(),
                    duration_secs = r.duration.as_secs_f64(),
                    dump_bytes = r.sizes.map(|s| s.dump),
                    snapshot_bytes = r.sizes.map(|s| s.snapshot),
                    media_bytes = r.sizes.map(|s| s.media),
                    $message
                )
            };
        }
        match r.outcome {
            Outcome::Unchanged | Outcome::Refreshed => outcome_line!(info, "refresh ended"),
            Outcome::Refused(_) => outcome_line!(warn, "refresh ended"),
            Outcome::Failed(_) => outcome_line!(error, "refresh ended"),
            Outcome::Removed => outcome_line!(warn, "project withdrawn: it is no longer allowlisted"),
        }
    }
}
